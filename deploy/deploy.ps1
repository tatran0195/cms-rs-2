<#
.SYNOPSIS
Deploy script for CMS Server

.DESCRIPTION
This script deploys a new version of the CMS server binary to the Windows server,
managing the NSSM service state in the process.

.PARAMETER Environment
The environment to deploy to (e.g. production)
#>

param (
    [Parameter(Mandatory=$false)]
    [string]$Environment = "production",
    
    [Parameter(Mandatory=$false)]
    [string]$ServiceName = "CMSServer"
)

$ErrorActionPreference = "Stop"

# Paths
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$WorkspaceRoot = (Resolve-Path "$ScriptDir\..").Path
$SourceBinary = "$WorkspaceRoot\target\release\cms_server.exe"
$DeployDir = "C:\CMS_Server"
$TargetBinary = "$DeployDir\cms_server.exe"

Write-Host "Starting deployment to $Environment environment..."

# Ensure target directory exists
if (-not (Test-Path $DeployDir)) {
    Write-Host "Creating deployment directory: $DeployDir"
    New-Item -ItemType Directory -Force -Path $DeployDir | Out-Null
}

# Check if NSSM is installed
if (-not (Get-Command nssm -ErrorAction SilentlyContinue)) {
    Write-Warning "NSSM is not in PATH. Assuming service is not running."
    $ServiceExists = $false
} else {
    $ServiceStatus = (nssm status $ServiceName 2>&1)
    if ($ServiceStatus -match "Can't open service") {
        $ServiceExists = $false
    } else {
        $ServiceExists = $true
    }
}

if ($ServiceExists) {
    Write-Host "Stopping service $ServiceName..."
    nssm stop $ServiceName
    # Wait for service to stop
    Start-Sleep -Seconds 5
}

Write-Host "Copying binary to $TargetBinary..."
if (Test-Path $SourceBinary) {
    Copy-Item -Path $SourceBinary -Destination $TargetBinary -Force
} else {
    Write-Error "Source binary not found at $SourceBinary. Build failed?"
}

if ($ServiceExists) {
    Write-Host "Starting service $ServiceName..."
    nssm start $ServiceName
    Write-Host "Deployment completed successfully!"
} else {
    Write-Warning "Service $ServiceName is not installed."
    Write-Host "To install the service, run deploy\nssm-install.bat"
}
