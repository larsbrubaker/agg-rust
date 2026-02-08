# Pre-commit check script for AGG Rust Port
# This script runs file length validation and other checks before commits

param(
    [switch]$Fix = $false
)

Write-Host "Running pre-commit checks for AGG Rust Port..." -ForegroundColor Cyan

# Function to check if a command exists
function Test-Command($cmdname) {
    return [bool](Get-Command -Name $cmdname -ErrorAction SilentlyContinue)
}

# Check if cargo is available
if (!(Test-Command "cargo")) {
    Write-Host "Cargo is not installed or not in PATH" -ForegroundColor Red
    exit 1
}

$exitCode = 0

Write-Host "`nRunning unit tests..." -ForegroundColor Yellow
try {
    cargo test --lib --quiet
    if ($LASTEXITCODE -eq 0) {
        Write-Host "All unit tests passed" -ForegroundColor Green
    } else {
        Write-Host "Some unit tests failed" -ForegroundColor Red
        $exitCode = 1
    }
} catch {
    Write-Host "Unit tests failed: $_" -ForegroundColor Red
    $exitCode = 1
}

# Skip integration tests if they don't exist
if (Test-Path "tests") {
    $integrationTests = Get-ChildItem -Path "tests" -Filter "*.rs"
    if ($integrationTests.Count -gt 0) {
        Write-Host "`nRunning integration tests..." -ForegroundColor Yellow
        try {
            cargo test --test "*" --quiet
            if ($LASTEXITCODE -eq 0) {
                Write-Host "Integration tests passed" -ForegroundColor Green
            } else {
                Write-Host "Integration tests failed" -ForegroundColor Red
                $exitCode = 1
            }
        } catch {
            Write-Host "Integration tests failed: $_" -ForegroundColor Red
            $exitCode = 1
        }
    }
}

if (Test-Command "cargo-fmt") {
    Write-Host "`nChecking code formatting..." -ForegroundColor Yellow
    try {
        cargo fmt --all -- --check
        if ($LASTEXITCODE -eq 0) {
            Write-Host "Code formatting is correct" -ForegroundColor Green
        } else {
            Write-Host "Code formatting issues found" -ForegroundColor Red
            if ($Fix) {
                Write-Host "Fixing code formatting..." -ForegroundColor Yellow
                cargo fmt --all
                Write-Host "Code formatting fixed" -ForegroundColor Green
            } else {
                Write-Host "   Run 'cargo fmt --all' to fix formatting or use -Fix flag" -ForegroundColor Yellow
                $exitCode = 1
            }
        }
    } catch {
        Write-Host "Code formatting check failed: $_" -ForegroundColor Red
        $exitCode = 1
    }
} else {
    Write-Host "rustfmt not available, skipping formatting check" -ForegroundColor Yellow
}

if (Test-Command "cargo-clippy") {
    Write-Host "`nRunning clippy lints..." -ForegroundColor Yellow
    try {
        cargo clippy --all-targets --all-features -- -D warnings
        if ($LASTEXITCODE -eq 0) {
            Write-Host "No clippy warnings found" -ForegroundColor Green
        } else {
            Write-Host "Clippy warnings found" -ForegroundColor Red
            $exitCode = 1
        }
    } catch {
        Write-Host "Clippy check failed: $_" -ForegroundColor Red
        $exitCode = 1
    }
} else {
    Write-Host "clippy not available, skipping lint check" -ForegroundColor Yellow
}

Write-Host "`nRunning build check..." -ForegroundColor Yellow
try {
    cargo build --all-targets
    if ($LASTEXITCODE -eq 0) {
        Write-Host "Build successful" -ForegroundColor Green
    } else {
        Write-Host "Build failed" -ForegroundColor Red
        $exitCode = 1
    }
} catch {
    Write-Host "Build check failed: $_" -ForegroundColor Red
    $exitCode = 1
}

Write-Host "`n" -NoNewline
if ($exitCode -eq 0) {
    Write-Host "All pre-commit checks passed!" -ForegroundColor Green
    Write-Host "   Your AGG Rust code is ready for commit." -ForegroundColor Green
} else {
    Write-Host "Pre-commit checks failed!" -ForegroundColor Red
    Write-Host "   Please fix the issues above before committing." -ForegroundColor Red
    Write-Host "`nHelpful commands:" -ForegroundColor Cyan
    Write-Host "   cargo test --verbose                    - Run tests with detailed output" -ForegroundColor White
    Write-Host "   cargo fmt --all                         - Fix formatting issues" -ForegroundColor White
    Write-Host "   cargo clippy --fix --all-targets        - Fix clippy warnings automatically" -ForegroundColor White
}

exit $exitCode
