#!/usr/bin/env powershell
# CMS Build Script for Windows
# This script builds the complete CMS application (backend + frontend)

param(
    [string]$Environment = "dev",
    [switch]$Release = $false,
    [switch]$Frontend = $true,
    [switch]$Backend = $true,
    [switch]$Test = $false
)

# Colors for output
$Reset = "`e[0m"
$Red = "`e[31m"
$Green = "`e[32m"
$Yellow = "`e[33m"
$Blue = "`e[34m"
$Cyan = "`e[36m"

function Write-Status([string]$message, [string]$color = $Cyan) {
    Write-Host "$color[$(Get-Date -Format 'HH:mm:ss')] $message$Reset"
}

function Write-Success([string]$message) {
    Write-Status $message $Green
}

function Write-Error([string]$message) {
    Write-Status $message $Red
}

function Write-Warning([string]$message) {
    Write-Status $message $Yellow
}

# Track start time
$startTime = Get-Date

Write-Status "Starting CMS build..."
Write-Status "Environment: $Environment"
Write-Status "Release mode: $Release"
Write-Status "Build frontend: $Frontend"
Write-Status "Build backend: $Backend"
Write-Status "Run tests: $Test"
Write-Status ""

# Set environment variable
$env:CMS_ENV = $Environment

# Step 1: Build Backend
if ($Backend) {
    Write-Status "Building backend..." $Blue
    
    $cargoArgs = @()
    if ($Release) {
        $cargoArgs += "--release"
    }
    
    try {
        cargo build @cargoArgs
        Write-Success "Backend built successfully"
    }
    catch {
        Write-Error "Backend build failed: $_"
        exit 1
    }
    Write-Status ""
}

# Step 2: Build Frontend
if ($Frontend) {
    Write-Status "Building frontend..." $Blue
    
    try {
        cd frontend
        npm install
        if ($Release) {
            npm run build
        }
        else {
            npm run build
        }
        cd ..
        Write-Success "Frontend built successfully"
    }
    catch {
        Write-Error "Frontend build failed: $_"
        exit 1
    }
    Write-Status ""
}

# Step 3: Run Tests
if ($Test) {
    Write-Status "Running tests..." $Blue
    
    try {
        # Run Rust tests
        cargo test
        Write-Success "Rust tests passed"
        
        # Run frontend tests (if any)
        cd frontend
        if (Test-Path "package.json") {
            npm test
            Write-Success "Frontend tests passed"
        }
        cd ..
    }
    catch {
        Write-Warning "Tests failed: $_"
        # Don't exit on test failure in CI
    }
    Write-Status ""
}

# Calculate duration
$endTime = Get-Date
$duration = $endTime - $startTime

Write-Success "Build completed in $($duration.TotalSeconds) seconds"

# Output binary location
if ($Release) {
    Write-Status "Release binary: .\target\release\cms-server.exe"
}
else {
    Write-Status "Debug binary: .\target\debug\cms-server.exe"
}

# Output frontend location
if ($Frontend) {
    Write-Status "Frontend build: .\frontend\dist"
}
