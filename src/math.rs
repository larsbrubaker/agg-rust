//! Geometric math utilities.
//!
//! Port of `agg_math.h` and `agg_sqrt_tables.cpp` — distances, intersections,
//! cross products, triangle operations, fast integer sqrt, and Bessel functions.

// ============================================================================
// Constants
// ============================================================================

/// Coinciding points maximal distance (epsilon).
pub const VERTEX_DIST_EPSILON: f64 = 1e-14;

/// Epsilon for intersection calculations.
pub const INTERSECTION_EPSILON: f64 = 1.0e-30;

// ============================================================================
// Cross product and point-in-triangle
// ============================================================================

/// Cross product of vectors (x2-x1, y2-y1) and (x-x2, y-y2).
/// The sign indicates which side of the line (x1,y1)→(x2,y2) the point (x,y) is on.
#[inline]
pub fn cross_product(x1: f64, y1: f64, x2: f64, y2: f64, x: f64, y: f64) -> f64 {
    (x - x2) * (y2 - y1) - (y - y2) * (x2 - x1)
}

/// Test if point (x, y) is inside triangle (x1,y1), (x2,y2), (x3,y3).
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn point_in_triangle(
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    x3: f64,
    y3: f64,
    x: f64,
    y: f64,
) -> bool {
    let cp1 = cross_product(x1, y1, x2, y2, x, y) < 0.0;
    let cp2 = cross_product(x2, y2, x3, y3, x, y) < 0.0;
    let cp3 = cross_product(x3, y3, x1, y1, x, y) < 0.0;
    cp1 == cp2 && cp2 == cp3 && cp3 == cp1
}

// ============================================================================
// Distance calculations
// ============================================================================

/// Euclidean distance between two points.
#[inline]
pub fn calc_distance(x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
    let dx = x2 - x1;
    let dy = y2 - y1;
    (dx * dx + dy * dy).sqrt()
}

/// Squared Euclidean distance between two points.
#[inline]
pub fn calc_sq_distance(x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
    let dx = x2 - x1;
    let dy = y2 - y1;
    dx * dx + dy * dy
}

/// Signed distance from point (x, y) to the infinite line through (x1,y1)→(x2,y2).
/// Positive means left side, negative means right side.
/// If the line segment is degenerate (length < VERTEX_DIST_EPSILON), returns
/// the distance from (x,y) to (x1,y1).
#[inline]
pub fn calc_line_point_distance(x1: f64, y1: f64, x2: f64, y2: f64, x: f64, y: f64) -> f64 {
    let dx = x2 - x1;
    let dy = y2 - y1;
    let d = (dx * dx + dy * dy).sqrt();
    if d < VERTEX_DIST_EPSILON {
        return calc_distance(x1, y1, x, y);
    }
    ((x - x2) * dy - (y - y2) * dx) / d
}

/// Compute the parameter `u` for the projection of point (x, y) onto
/// the line segment (x1,y1)→(x2,y2). Returns 0 if the segment is degenerate.
#[inline]
pub fn calc_segment_point_u(x1: f64, y1: f64, x2: f64, y2: f64, x: f64, y: f64) -> f64 {
    let dx = x2 - x1;
    let dy = y2 - y1;

    if dx == 0.0 && dy == 0.0 {
        return 0.0;
    }

    let pdx = x - x1;
    let pdy = y - y1;

    (pdx * dx + pdy * dy) / (dx * dx + dy * dy)
}

/// Squared distance from point (x, y) to the closest point on segment
/// (x1,y1)→(x2,y2), given pre-computed parameter `u`.
#[inline]
pub fn calc_segment_point_sq_distance_with_u(
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    x: f64,
    y: f64,
    u: f64,
) -> f64 {
    if u <= 0.0 {
        calc_sq_distance(x, y, x1, y1)
    } else if u >= 1.0 {
        calc_sq_distance(x, y, x2, y2)
    } else {
        calc_sq_distance(x, y, x1 + u * (x2 - x1), y1 + u * (y2 - y1))
    }
}

/// Squared distance from point (x, y) to the closest point on segment
/// (x1,y1)→(x2,y2).
#[inline]
pub fn calc_segment_point_sq_distance(x1: f64, y1: f64, x2: f64, y2: f64, x: f64, y: f64) -> f64 {
    calc_segment_point_sq_distance_with_u(
        x1,
        y1,
        x2,
        y2,
        x,
        y,
        calc_segment_point_u(x1, y1, x2, y2, x, y),
    )
}

// ============================================================================
// Intersection
// ============================================================================

/// Calculate the intersection point of two line segments:
/// (ax,ay)→(bx,by) and (cx,cy)→(dx,dy).
/// Returns `Some((x, y))` if they intersect, `None` if parallel.
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn calc_intersection(
    ax: f64,
    ay: f64,
    bx: f64,
    by: f64,
    cx: f64,
    cy: f64,
    dx: f64,
    dy: f64,
) -> Option<(f64, f64)> {
    let num = (ay - cy) * (dx - cx) - (ax - cx) * (dy - cy);
    let den = (bx - ax) * (dy - cy) - (by - ay) * (dx - cx);
    if den.abs() < INTERSECTION_EPSILON {
        return None;
    }
    let r = num / den;
    Some((ax + r * (bx - ax), ay + r * (by - ay)))
}

/// Quick check whether two line segments (x1,y1)→(x2,y2) and
/// (x3,y3)→(x4,y4) intersect (boundary excluded).
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn intersection_exists(
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    x3: f64,
    y3: f64,
    x4: f64,
    y4: f64,
) -> bool {
    let dx1 = x2 - x1;
    let dy1 = y2 - y1;
    let dx2 = x4 - x3;
    let dy2 = y4 - y3;
    ((x3 - x2) * dy1 - (y3 - y2) * dx1 < 0.0) != ((x4 - x2) * dy1 - (y4 - y2) * dx1 < 0.0)
        && ((x1 - x4) * dy2 - (y1 - y4) * dx2 < 0.0) != ((x2 - x4) * dy2 - (y2 - y4) * dx2 < 0.0)
}

// ============================================================================
// Orthogonal and triangle operations
// ============================================================================

/// Calculate the orthogonal displacement vector of magnitude `thickness`
/// perpendicular to the line (x1,y1)→(x2,y2).
#[inline]
pub fn calc_orthogonal(thickness: f64, x1: f64, y1: f64, x2: f64, y2: f64) -> (f64, f64) {
    let dx = x2 - x1;
    let dy = y2 - y1;
    let d = (dx * dx + dy * dy).sqrt();
    (thickness * dy / d, -thickness * dx / d)
}

/// Dilate a triangle by distance `d`, producing 6 output points
/// (two per edge). Returns `([x0..x5], [y0..y5])`.
pub fn dilate_triangle(
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    x3: f64,
    y3: f64,
    d: f64,
) -> ([f64; 6], [f64; 6]) {
    let mut dx1 = 0.0;
    let mut dy1 = 0.0;
    let mut dx2 = 0.0;
    let mut dy2 = 0.0;
    let mut dx3 = 0.0;
    let mut dy3 = 0.0;
    let mut d = d;
    let loc = cross_product(x1, y1, x2, y2, x3, y3);
    if loc.abs() > INTERSECTION_EPSILON {
        if cross_product(x1, y1, x2, y2, x3, y3) > 0.0 {
            d = -d;
        }
        let o1 = calc_orthogonal(d, x1, y1, x2, y2);
        dx1 = o1.0;
        dy1 = o1.1;
        let o2 = calc_orthogonal(d, x2, y2, x3, y3);
        dx2 = o2.0;
        dy2 = o2.1;
        let o3 = calc_orthogonal(d, x3, y3, x1, y1);
        dx3 = o3.0;
        dy3 = o3.1;
    }
    let x = [x1 + dx1, x2 + dx1, x2 + dx2, x3 + dx2, x3 + dx3, x1 + dx3];
    let y = [y1 + dy1, y2 + dy1, y2 + dy2, y3 + dy2, y3 + dy3, y1 + dy3];
    (x, y)
}

/// Signed area of triangle (x1,y1), (x2,y2), (x3,y3).
#[inline]
pub fn calc_triangle_area(x1: f64, y1: f64, x2: f64, y2: f64, x3: f64, y3: f64) -> f64 {
    (x1 * y2 - x2 * y1 + x2 * y3 - x3 * y2 + x3 * y1 - x1 * y3) * 0.5
}

/// Signed area of a polygon defined by a slice of points with `x` and `y` fields.
pub fn calc_polygon_area(vertices: &[crate::basics::PointD]) -> f64 {
    if vertices.is_empty() {
        return 0.0;
    }
    let mut sum = 0.0;
    let mut x = vertices[0].x;
    let mut y = vertices[0].y;
    let xs = x;
    let ys = y;

    for v in &vertices[1..] {
        sum += x * v.y - y * v.x;
        x = v.x;
        y = v.y;
    }
    (sum + x * ys - y * xs) * 0.5
}

/// Signed area of a polygon defined by a slice of `VertexDist`.
/// Same algorithm as `calc_polygon_area` but works with `VertexDist` slices
/// (needed by `vcgen_contour`).
pub fn calc_polygon_area_vd(vertices: &[crate::array::VertexDist]) -> f64 {
    if vertices.is_empty() {
        return 0.0;
    }
    let mut sum = 0.0;
    let mut x = vertices[0].x;
    let mut y = vertices[0].y;
    let xs = x;
    let ys = y;

    for v in &vertices[1..] {
        sum += x * v.y - y * v.x;
        x = v.x;
        y = v.y;
    }
    (sum + x * ys - y * xs) * 0.5
}

// ============================================================================
// Fast integer square root (lookup table based)
// ============================================================================

/// Lookup table for fast integer sqrt. 1024 entries.
/// Port of `g_sqrt_table` from `agg_sqrt_tables.cpp`.
#[rustfmt::skip]
static SQRT_TABLE: [u16; 1024] = [
    0,
    2048,2896,3547,4096,4579,5017,5418,5793,6144,6476,6792,7094,7384,7663,7932,8192,8444,
    8689,8927,9159,9385,9606,9822,10033,10240,10443,10642,10837,11029,11217,11403,11585,
    11765,11942,12116,12288,12457,12625,12790,12953,13114,13273,13430,13585,13738,13890,
    14040,14189,14336,14482,14626,14768,14910,15050,15188,15326,15462,15597,15731,15864,
    15995,16126,16255,16384,16512,16638,16764,16888,17012,17135,17257,17378,17498,17618,
    17736,17854,17971,18087,18203,18318,18432,18545,18658,18770,18882,18992,19102,19212,
    19321,19429,19537,19644,19750,19856,19961,20066,20170,20274,20377,20480,20582,20684,
    20785,20886,20986,21085,21185,21283,21382,21480,21577,21674,21771,21867,21962,22058,
    22153,22247,22341,22435,22528,22621,22713,22806,22897,22989,23080,23170,23261,23351,
    23440,23530,23619,23707,23796,23884,23971,24059,24146,24232,24319,24405,24491,24576,
    24661,24746,24831,24915,24999,25083,25166,25249,25332,25415,25497,25580,25661,25743,
    25824,25905,25986,26067,26147,26227,26307,26387,26466,26545,26624,26703,26781,26859,
    26937,27015,27092,27170,27247,27324,27400,27477,27553,27629,27705,27780,27856,27931,
    28006,28081,28155,28230,28304,28378,28452,28525,28599,28672,28745,28818,28891,28963,
    29035,29108,29180,29251,29323,29394,29466,29537,29608,29678,29749,29819,29890,29960,
    30030,30099,30169,30238,30308,30377,30446,30515,30583,30652,30720,30788,30856,30924,
    30992,31059,31127,31194,31261,31328,31395,31462,31529,31595,31661,31727,31794,31859,
    31925,31991,32056,32122,32187,32252,32317,32382,32446,32511,32575,32640,32704,32768,
    32832,32896,32959,33023,33086,33150,33213,33276,33339,33402,33465,33527,33590,33652,
    33714,33776,33839,33900,33962,34024,34086,34147,34208,34270,34331,34392,34453,34514,
    34574,34635,34695,34756,34816,34876,34936,34996,35056,35116,35176,35235,35295,35354,
    35413,35472,35531,35590,35649,35708,35767,35825,35884,35942,36001,36059,36117,36175,
    36233,36291,36348,36406,36464,36521,36578,36636,36693,36750,36807,36864,36921,36978,
    37034,37091,37147,37204,37260,37316,37372,37429,37485,37540,37596,37652,37708,37763,
    37819,37874,37929,37985,38040,38095,38150,38205,38260,38315,38369,38424,38478,38533,
    38587,38642,38696,38750,38804,38858,38912,38966,39020,39073,39127,39181,39234,39287,
    39341,39394,39447,39500,39553,39606,39659,39712,39765,39818,39870,39923,39975,40028,
    40080,40132,40185,40237,40289,40341,40393,40445,40497,40548,40600,40652,40703,40755,
    40806,40857,40909,40960,41011,41062,41113,41164,41215,41266,41317,41368,41418,41469,
    41519,41570,41620,41671,41721,41771,41821,41871,41922,41972,42021,42071,42121,42171,
    42221,42270,42320,42369,42419,42468,42518,42567,42616,42665,42714,42763,42813,42861,
    42910,42959,43008,43057,43105,43154,43203,43251,43300,43348,43396,43445,43493,43541,
    43589,43637,43685,43733,43781,43829,43877,43925,43972,44020,44068,44115,44163,44210,
    44258,44305,44352,44400,44447,44494,44541,44588,44635,44682,44729,44776,44823,44869,
    44916,44963,45009,45056,45103,45149,45195,45242,45288,45334,45381,45427,45473,45519,
    45565,45611,45657,45703,45749,45795,45840,45886,45932,45977,46023,46069,46114,46160,
    46205,46250,46296,46341,46386,46431,46477,46522,46567,46612,46657,46702,46746,46791,
    46836,46881,46926,46970,47015,47059,47104,47149,47193,47237,47282,47326,47370,47415,
    47459,47503,47547,47591,47635,47679,47723,47767,47811,47855,47899,47942,47986,48030,
    48074,48117,48161,48204,48248,48291,48335,48378,48421,48465,48508,48551,48594,48637,
    48680,48723,48766,48809,48852,48895,48938,48981,49024,49067,49109,49152,49195,49237,
    49280,49322,49365,49407,49450,49492,49535,49577,49619,49661,49704,49746,49788,49830,
    49872,49914,49956,49998,50040,50082,50124,50166,50207,50249,50291,50332,50374,50416,
    50457,50499,50540,50582,50623,50665,50706,50747,50789,50830,50871,50912,50954,50995,
    51036,51077,51118,51159,51200,51241,51282,51323,51364,51404,51445,51486,51527,51567,
    51608,51649,51689,51730,51770,51811,51851,51892,51932,51972,52013,52053,52093,52134,
    52174,52214,52254,52294,52334,52374,52414,52454,52494,52534,52574,52614,52654,52694,
    52734,52773,52813,52853,52892,52932,52972,53011,53051,53090,53130,53169,53209,53248,
    53287,53327,53366,53405,53445,53484,53523,53562,53601,53640,53679,53719,53758,53797,
    53836,53874,53913,53952,53991,54030,54069,54108,54146,54185,54224,54262,54301,54340,
    54378,54417,54455,54494,54532,54571,54609,54647,54686,54724,54762,54801,54839,54877,
    54915,54954,54992,55030,55068,55106,55144,55182,55220,55258,55296,55334,55372,55410,
    55447,55485,55523,55561,55599,55636,55674,55712,55749,55787,55824,55862,55900,55937,
    55975,56012,56049,56087,56124,56162,56199,56236,56273,56311,56348,56385,56422,56459,
    56497,56534,56571,56608,56645,56682,56719,56756,56793,56830,56867,56903,56940,56977,
    57014,57051,57087,57124,57161,57198,57234,57271,57307,57344,57381,57417,57454,57490,
    57527,57563,57599,57636,57672,57709,57745,57781,57817,57854,57890,57926,57962,57999,
    58035,58071,58107,58143,58179,58215,58251,58287,58323,58359,58395,58431,58467,58503,
    58538,58574,58610,58646,58682,58717,58753,58789,58824,58860,58896,58931,58967,59002,
    59038,59073,59109,59144,59180,59215,59251,59286,59321,59357,59392,59427,59463,59498,
    59533,59568,59603,59639,59674,59709,59744,59779,59814,59849,59884,59919,59954,59989,
    60024,60059,60094,60129,60164,60199,60233,60268,60303,60338,60373,60407,60442,60477,
    60511,60546,60581,60615,60650,60684,60719,60753,60788,60822,60857,60891,60926,60960,
    60995,61029,61063,61098,61132,61166,61201,61235,61269,61303,61338,61372,61406,61440,
    61474,61508,61542,61576,61610,61644,61678,61712,61746,61780,61814,61848,61882,61916,
    61950,61984,62018,62051,62085,62119,62153,62186,62220,62254,62287,62321,62355,62388,
    62422,62456,62489,62523,62556,62590,62623,62657,62690,62724,62757,62790,62824,62857,
    62891,62924,62957,62991,63024,63057,63090,63124,63157,63190,63223,63256,63289,63323,
    63356,63389,63422,63455,63488,63521,63554,63587,63620,63653,63686,63719,63752,63785,
    63817,63850,63883,63916,63949,63982,64014,64047,64080,64113,64145,64178,64211,64243,
    64276,64309,64341,64374,64406,64439,64471,64504,64536,64569,64601,64634,64666,64699,
    64731,64763,64796,64828,64861,64893,64925,64957,64990,65022,65054,65086,65119,65151,
    65183,65215,65247,65279,65312,65344,65376,65408,65440,65472,65504,
];

/// Lookup table mapping byte values to their most significant bit position.
/// Port of `g_elder_bit_table` from `agg_sqrt_tables.cpp`.
#[rustfmt::skip]
static ELDER_BIT_TABLE: [i8; 256] = [
    0,0,1,1,2,2,2,2,3,3,3,3,3,3,3,3,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,4,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,
    6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,
    6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,6,
    7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,
    7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,
    7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,
    7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,
];

/// Fast integer square root using lookup tables.
/// No divisions, multiplications, or loops — just bit shifts and table lookups.
/// Port of C++ `fast_sqrt` (portable C path).
pub fn fast_sqrt(val: u32) -> u32 {
    let t = val;
    let mut shift: i32 = 11;

    let bit: i32;
    let b = (t >> 24) as u8;
    if b != 0 {
        bit = ELDER_BIT_TABLE[b as usize] as i32 + 24;
    } else {
        let b = ((t >> 16) & 0xFF) as u8;
        if b != 0 {
            bit = ELDER_BIT_TABLE[b as usize] as i32 + 16;
        } else {
            let b = ((t >> 8) & 0xFF) as u8;
            if b != 0 {
                bit = ELDER_BIT_TABLE[b as usize] as i32 + 8;
            } else {
                bit = ELDER_BIT_TABLE[t as u8 as usize] as i32;
            }
        }
    }

    let mut val = val;
    let bit = bit - 9;
    if bit > 0 {
        let half_bit = (bit >> 1) + (bit & 1);
        shift -= half_bit;
        val >>= (half_bit << 1) as u32;
    }
    (SQRT_TABLE[val as usize] as u32) >> shift as u32
}

// ============================================================================
// Bessel function
// ============================================================================

/// Bessel function of the first kind of order `n`.
///
/// Adapted for AGG by Andy Wilk (castor.vulgaris@gmail.com).
/// Originally from C++ Mathematical Library (Gareth Walker).
pub fn besj(x: f64, n: i32) -> f64 {
    if n < 0 {
        return 0.0;
    }
    let d = 1e-6;
    let mut b = 0.0;
    if x.abs() <= d {
        if n != 0 {
            return 0.0;
        }
        return 1.0;
    }
    let mut b1 = 0.0;
    let mut m1 = (x.abs() + 6.0) as i32;
    if x.abs() > 5.0 {
        m1 = (1.4 * x.abs() + 60.0 / x.abs()) as i32;
    }
    let mut m2 = (n as f64 + 2.0 + x.abs() / 4.0) as i32;
    if m1 > m2 {
        m2 = m1;
    }

    loop {
        let mut c3 = 0.0;
        let mut c2 = 1e-30;
        let mut c4 = 0.0;
        let mut m8 = 1;
        if m2 / 2 * 2 == m2 {
            m8 = -1;
        }
        let imax = m2 - 2;
        for i in 1..=imax {
            let c6 = 2.0 * (m2 - i) as f64 * c2 / x - c3;
            c3 = c2;
            c2 = c6;
            if m2 - i - 1 == n {
                b = c6;
            }
            m8 = -m8;
            if m8 > 0 {
                c4 += 2.0 * c6;
            }
        }
        let c6 = 2.0 * c2 / x - c3;
        if n == 0 {
            b = c6;
        }
        c4 += c6;
        b /= c4;
        if (b - b1).abs() < d {
            return b;
        }
        b1 = b;
        m2 += 3;
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f64 = 1e-10;

    #[test]
    fn test_cross_product() {
        // Point on the line: cross product should be 0
        let cp = cross_product(0.0, 0.0, 1.0, 0.0, 2.0, 0.0);
        assert!(cp.abs() < EPSILON);

        // Point above the line (left side): negative cross product
        // Formula: (x-x2)*(y2-y1) - (y-y2)*(x2-x1) = (-0.5)*0 - (1.0)*(1.0) = -1.0
        let cp = cross_product(0.0, 0.0, 1.0, 0.0, 0.5, 1.0);
        assert!(cp < 0.0);

        // Point below the line (right side): positive cross product
        let cp = cross_product(0.0, 0.0, 1.0, 0.0, 0.5, -1.0);
        assert!(cp > 0.0);
    }

    #[test]
    fn test_point_in_triangle() {
        // Center of unit triangle
        assert!(point_in_triangle(0.0, 0.0, 1.0, 0.0, 0.5, 1.0, 0.5, 0.3));
        // Outside
        assert!(!point_in_triangle(0.0, 0.0, 1.0, 0.0, 0.5, 1.0, 2.0, 2.0));
    }

    #[test]
    fn test_calc_distance() {
        assert!((calc_distance(0.0, 0.0, 3.0, 4.0) - 5.0).abs() < EPSILON);
        assert!((calc_distance(0.0, 0.0, 0.0, 0.0)).abs() < EPSILON);
        assert!((calc_distance(1.0, 1.0, 1.0, 1.0)).abs() < EPSILON);
    }

    #[test]
    fn test_calc_sq_distance() {
        assert!((calc_sq_distance(0.0, 0.0, 3.0, 4.0) - 25.0).abs() < EPSILON);
    }

    #[test]
    fn test_calc_line_point_distance() {
        // Point (0, 1) relative to line (0,0)→(1,0):
        // ((x-x2)*dy - (y-y2)*dx) / d = ((0-1)*0 - (1-0)*1) / 1 = -1.0
        let d = calc_line_point_distance(0.0, 0.0, 1.0, 0.0, 0.0, 1.0);
        assert!((d - (-1.0)).abs() < EPSILON);

        // Point (0, -1) below the line: positive
        let d = calc_line_point_distance(0.0, 0.0, 1.0, 0.0, 0.0, -1.0);
        assert!((d - 1.0).abs() < EPSILON);
    }

    #[test]
    fn test_calc_segment_point_u() {
        // Midpoint of segment
        let u = calc_segment_point_u(0.0, 0.0, 2.0, 0.0, 1.0, 0.0);
        assert!((u - 0.5).abs() < EPSILON);

        // Before start
        let u = calc_segment_point_u(0.0, 0.0, 2.0, 0.0, -1.0, 0.0);
        assert!(u < 0.0);

        // After end
        let u = calc_segment_point_u(0.0, 0.0, 2.0, 0.0, 3.0, 0.0);
        assert!(u > 1.0);

        // Degenerate segment
        let u = calc_segment_point_u(0.0, 0.0, 0.0, 0.0, 1.0, 1.0);
        assert_eq!(u, 0.0);
    }

    #[test]
    fn test_calc_segment_point_sq_distance() {
        // Distance to midpoint of horizontal segment from point above
        let d = calc_segment_point_sq_distance(0.0, 0.0, 2.0, 0.0, 1.0, 1.0);
        assert!((d - 1.0).abs() < EPSILON);

        // Distance to start (before segment)
        let d = calc_segment_point_sq_distance(0.0, 0.0, 2.0, 0.0, -1.0, 0.0);
        assert!((d - 1.0).abs() < EPSILON);
    }

    #[test]
    fn test_calc_intersection() {
        // Two perpendicular lines crossing at (1, 1)
        let result = calc_intersection(0.0, 1.0, 2.0, 1.0, 1.0, 0.0, 1.0, 2.0);
        assert!(result.is_some());
        let (x, y) = result.unwrap();
        assert!((x - 1.0).abs() < EPSILON);
        assert!((y - 1.0).abs() < EPSILON);

        // Parallel lines
        let result = calc_intersection(0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0);
        assert!(result.is_none());
    }

    #[test]
    fn test_intersection_exists() {
        // Crossing segments
        assert!(intersection_exists(0.0, 0.0, 2.0, 2.0, 0.0, 2.0, 2.0, 0.0));

        // Non-crossing segments
        assert!(!intersection_exists(0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0));
    }

    #[test]
    fn test_calc_triangle_area() {
        // Unit right triangle: area = 0.5
        let area = calc_triangle_area(0.0, 0.0, 1.0, 0.0, 0.0, 1.0);
        assert!((area - 0.5).abs() < EPSILON);

        // Reversed winding: negative area
        let area = calc_triangle_area(0.0, 0.0, 0.0, 1.0, 1.0, 0.0);
        assert!((area - (-0.5)).abs() < EPSILON);
    }

    #[test]
    fn test_calc_polygon_area() {
        use crate::basics::PointD;
        // Unit square: area = 1.0
        let square = vec![
            PointD::new(0.0, 0.0),
            PointD::new(1.0, 0.0),
            PointD::new(1.0, 1.0),
            PointD::new(0.0, 1.0),
        ];
        let area = calc_polygon_area(&square);
        assert!((area - 1.0).abs() < EPSILON);
    }

    #[test]
    fn test_calc_polygon_area_vd() {
        use crate::array::VertexDist;
        // Unit square: area = 1.0
        let square = vec![
            VertexDist::new(0.0, 0.0),
            VertexDist::new(1.0, 0.0),
            VertexDist::new(1.0, 1.0),
            VertexDist::new(0.0, 1.0),
        ];
        let area = calc_polygon_area_vd(&square);
        assert!((area - 1.0).abs() < EPSILON);

        // Empty: area = 0
        let empty: Vec<VertexDist> = vec![];
        assert_eq!(calc_polygon_area_vd(&empty), 0.0);
    }

    #[test]
    fn test_calc_polygon_area_vd_ccw() {
        use crate::array::VertexDist;
        // CCW square: area = -1.0
        let square = vec![
            VertexDist::new(0.0, 0.0),
            VertexDist::new(0.0, 1.0),
            VertexDist::new(1.0, 1.0),
            VertexDist::new(1.0, 0.0),
        ];
        let area = calc_polygon_area_vd(&square);
        assert!((area - (-1.0)).abs() < EPSILON);
    }

    #[test]
    fn test_fast_sqrt() {
        // Test a few known values
        assert_eq!(fast_sqrt(0), 0);
        assert_eq!(fast_sqrt(1), 1);
        assert_eq!(fast_sqrt(4), 2);
        assert_eq!(fast_sqrt(9), 3);
        assert_eq!(fast_sqrt(16), 4);
        assert_eq!(fast_sqrt(100), 10);
        assert_eq!(fast_sqrt(10000), 100);
    }

    #[test]
    fn test_fast_sqrt_accuracy() {
        // fast_sqrt should be reasonably accurate for values used in AGG
        for val in [25, 49, 64, 81, 144, 225, 400, 625, 900, 1600, 2500, 10000] {
            let expected = (val as f64).sqrt().round() as u32;
            let result = fast_sqrt(val);
            assert_eq!(
                result, expected,
                "fast_sqrt({}) = {}, expected {}",
                val, result, expected
            );
        }
    }

    #[test]
    fn test_besj_order_zero() {
        // J_0(0) = 1
        assert!((besj(0.0, 0) - 1.0).abs() < 1e-5);
        // J_0(2.4048...) ≈ 0 (first zero)
        assert!(besj(2.4048, 0).abs() < 0.001);
    }

    #[test]
    fn test_besj_order_one() {
        // J_1(0) = 0
        assert!((besj(0.0, 1)).abs() < 1e-5);
        // J_1(3.8317...) ≈ 0 (first zero)
        assert!(besj(3.8317, 1).abs() < 0.001);
    }

    #[test]
    fn test_besj_negative_order() {
        assert_eq!(besj(1.0, -1), 0.0);
    }

    #[test]
    fn test_calc_orthogonal() {
        let (dx, dy) = calc_orthogonal(1.0, 0.0, 0.0, 1.0, 0.0);
        assert!((dx).abs() < EPSILON);
        assert!((dy - (-1.0)).abs() < EPSILON);
    }

    #[test]
    fn test_dilate_triangle() {
        // Just verify it doesn't panic and produces 6 points
        let (x, y) = dilate_triangle(0.0, 0.0, 1.0, 0.0, 0.5, 1.0, 0.1);
        assert_eq!(x.len(), 6);
        assert_eq!(y.len(), 6);
    }
}
