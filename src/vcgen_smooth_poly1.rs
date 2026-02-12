//! Smooth polygon vertex generator.
//!
//! Port of `agg_vcgen_smooth_poly1.h` / `agg_vcgen_smooth_poly1.cpp`.

use crate::array::{VertexDist, VertexSequence};
use crate::basics::{
    get_close_flag, is_move_to, is_stop, is_vertex, PATH_CMD_CURVE3, PATH_CMD_CURVE4,
    PATH_CMD_END_POLY, PATH_CMD_LINE_TO, PATH_CMD_MOVE_TO, PATH_CMD_STOP,
};
use crate::conv_adaptor_vcgen::VcgenGenerator;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Status {
    Initial,
    Ready,
    Polygon,
    CtrlB,
    CtrlE,
    Ctrl1,
    Ctrl2,
    EndPoly,
    Stop,
}

/// Port of C++ `vcgen_smooth_poly1`.
pub struct VcgenSmoothPoly1 {
    src_vertices: VertexSequence,
    smooth_value: f64,
    closed: u32,
    status: Status,
    src_vertex: usize,
    ctrl1_x: f64,
    ctrl1_y: f64,
    ctrl2_x: f64,
    ctrl2_y: f64,
}

impl VcgenSmoothPoly1 {
    pub fn new() -> Self {
        Self {
            src_vertices: VertexSequence::new(),
            smooth_value: 0.5,
            closed: 0,
            status: Status::Initial,
            src_vertex: 0,
            ctrl1_x: 0.0,
            ctrl1_y: 0.0,
            ctrl2_x: 0.0,
            ctrl2_y: 0.0,
        }
    }

    pub fn set_smooth_value(&mut self, v: f64) {
        self.smooth_value = v * 0.5;
    }

    pub fn smooth_value(&self) -> f64 {
        self.smooth_value * 2.0
    }

    pub fn remove_all(&mut self) {
        self.src_vertices.remove_all();
        self.closed = 0;
        self.status = Status::Initial;
    }

    pub fn add_vertex(&mut self, x: f64, y: f64, cmd: u32) {
        self.status = Status::Initial;
        if is_move_to(cmd) {
            self.src_vertices.modify_last(VertexDist::new(x, y));
        } else if is_vertex(cmd) {
            self.src_vertices.add(VertexDist::new(x, y));
        } else {
            self.closed = get_close_flag(cmd);
        }
    }

    pub fn rewind(&mut self, _path_id: u32) {
        if self.status == Status::Initial {
            self.src_vertices.close(self.closed != 0);
        }
        self.status = Status::Ready;
        self.src_vertex = 0;
    }

    fn calculate(&mut self, v0: &VertexDist, v1: &VertexDist, v2: &VertexDist, v3: &VertexDist) {
        let k1 = v0.dist / (v0.dist + v1.dist);
        let k2 = v1.dist / (v1.dist + v2.dist);

        let xm1 = v0.x + (v2.x - v0.x) * k1;
        let ym1 = v0.y + (v2.y - v0.y) * k1;
        let xm2 = v1.x + (v3.x - v1.x) * k2;
        let ym2 = v1.y + (v3.y - v1.y) * k2;

        self.ctrl1_x = v1.x + self.smooth_value * (v2.x - xm1);
        self.ctrl1_y = v1.y + self.smooth_value * (v2.y - ym1);
        self.ctrl2_x = v2.x + self.smooth_value * (v1.x - xm2);
        self.ctrl2_y = v2.y + self.smooth_value * (v1.y - ym2);
    }

    pub fn vertex(&mut self, x: &mut f64, y: &mut f64) -> u32 {
        let mut cmd = PATH_CMD_LINE_TO;
        loop {
            if is_stop(cmd) {
                return cmd;
            }
            match self.status {
                Status::Initial => {
                    self.rewind(0);
                    continue;
                }
                Status::Ready => {
                    if self.src_vertices.size() < 2 {
                        cmd = PATH_CMD_STOP;
                        continue;
                    }
                    if self.src_vertices.size() == 2 {
                        *x = self.src_vertices[self.src_vertex].x;
                        *y = self.src_vertices[self.src_vertex].y;
                        self.src_vertex += 1;
                        if self.src_vertex == 1 {
                            return PATH_CMD_MOVE_TO;
                        }
                        if self.src_vertex == 2 {
                            return PATH_CMD_LINE_TO;
                        }
                        cmd = PATH_CMD_STOP;
                        continue;
                    }
                    cmd = PATH_CMD_MOVE_TO;
                    self.status = Status::Polygon;
                    self.src_vertex = 0;
                    continue;
                }
                Status::Polygon => {
                    if self.closed != 0 {
                        if self.src_vertex >= self.src_vertices.size() {
                            *x = self.src_vertices[0].x;
                            *y = self.src_vertices[0].y;
                            self.status = Status::EndPoly;
                            return PATH_CMD_CURVE4;
                        }
                    } else if self.src_vertex >= self.src_vertices.size() - 1 {
                        *x = self.src_vertices[self.src_vertices.size() - 1].x;
                        *y = self.src_vertices[self.src_vertices.size() - 1].y;
                        self.status = Status::EndPoly;
                        return PATH_CMD_CURVE3;
                    }

                    let v0 = *self.src_vertices.prev(self.src_vertex);
                    let v1 = *self.src_vertices.curr(self.src_vertex);
                    let v2 = *self.src_vertices.next(self.src_vertex);
                    let v3 = *self.src_vertices.next(self.src_vertex + 1);
                    self.calculate(&v0, &v1, &v2, &v3);

                    *x = self.src_vertices[self.src_vertex].x;
                    *y = self.src_vertices[self.src_vertex].y;
                    self.src_vertex += 1;

                    if self.closed != 0 {
                        self.status = Status::Ctrl1;
                        return if self.src_vertex == 1 {
                            PATH_CMD_MOVE_TO
                        } else {
                            PATH_CMD_CURVE4
                        };
                    }
                    if self.src_vertex == 1 {
                        self.status = Status::CtrlB;
                        return PATH_CMD_MOVE_TO;
                    }
                    if self.src_vertex >= self.src_vertices.size() - 1 {
                        self.status = Status::CtrlE;
                        return PATH_CMD_CURVE3;
                    }
                    self.status = Status::Ctrl1;
                    return PATH_CMD_CURVE4;
                }
                Status::CtrlB => {
                    *x = self.ctrl2_x;
                    *y = self.ctrl2_y;
                    self.status = Status::Polygon;
                    return PATH_CMD_CURVE3;
                }
                Status::CtrlE => {
                    *x = self.ctrl1_x;
                    *y = self.ctrl1_y;
                    self.status = Status::Polygon;
                    return PATH_CMD_CURVE3;
                }
                Status::Ctrl1 => {
                    *x = self.ctrl1_x;
                    *y = self.ctrl1_y;
                    self.status = Status::Ctrl2;
                    return PATH_CMD_CURVE4;
                }
                Status::Ctrl2 => {
                    *x = self.ctrl2_x;
                    *y = self.ctrl2_y;
                    self.status = Status::Polygon;
                    return PATH_CMD_CURVE4;
                }
                Status::EndPoly => {
                    self.status = Status::Stop;
                    return PATH_CMD_END_POLY | self.closed;
                }
                Status::Stop => {
                    return PATH_CMD_STOP;
                }
            }
        }
    }
}

impl Default for VcgenSmoothPoly1 {
    fn default() -> Self {
        Self::new()
    }
}

impl VcgenGenerator for VcgenSmoothPoly1 {
    fn remove_all(&mut self) {
        self.remove_all();
    }
    fn add_vertex(&mut self, x: f64, y: f64, cmd: u32) {
        self.add_vertex(x, y, cmd);
    }
    fn rewind(&mut self, path_id: u32) {
        self.rewind(path_id);
    }
    fn vertex(&mut self, x: &mut f64, y: &mut f64) -> u32 {
        self.vertex(x, y)
    }
}
