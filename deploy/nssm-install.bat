@echo off
:: CMS Server - NSSM Installation Script
:: This script installs CMS as a Windows service using NSSM
::
:: Prerequisites:
::   - NSSM (Non-Sucking Service Manager) must be installed
::   - Node.js must be installed (for frontend build)
::   - Rust must be installed
::   - CMS binary must be built
::
:: Usage:
::   nssm-install.bat [service_name] [binary_path] [config_path]

SETLOCAL ENABLEDELAYEDEXPANSION

:: Default values
SET SERVICE_NAME=CMSServer
SET BINARY_PATH=%~dp0..\target\release\cms-server.exe
SET CONFIG_PATH=%~dp0..\config\deploy.env
SET FRONTEND_DIR=%~dp0..\dist\frontend

:: Parse command line arguments
IF NOT "%~1"=="" SET SERVICE_NAME=%~1
IF NOT "%~2"=="" SET BINARY_PATH=%~2
IF NOT "%~3"=="" SET CONFIG_PATH=%~3

:: Check if NSSM is available
WHERE nssm >nul 2>&1
IF %ERRORLEVEL% NEQ 0 (
    ECHO ERROR: NSSM is not installed or not in PATH
    ECHO Please download NSSM from: https://nssm.cc/download
    ECHO and add it to your PATH or place nssm.exe in this directory
    PAUSE
    EXIT /B 1
)

:: Check if binary exists
IF NOT EXIST "%BINARY_PATH%" (
    ECHO ERROR: Binary not found at: %BINARY_PATH%
    ECHO Please build the project first: cargo build --release
    PAUSE
    EXIT /B 1
)

:: Check if config exists
IF NOT EXIST "%CONFIG_PATH%" (
    ECHO WARNING: Config file not found at: %CONFIG_PATH%
    ECHO Using default configuration
    SET CONFIG_PATH=
)

ECHO Installing CMS service...
ECHO Service name: %SERVICE_NAME%
ECHO Binary path: %BINARY_PATH%
ECHO Config path: %CONFIG_PATH%
ECHO.

:: Install the service
nssm install %SERVICE_NAME% "%BINARY_PATH%" || (
    ECHO ERROR: Failed to install service
    PAUSE
    EXIT /B 1
)

:: Configure the service
nssm set %SERVICE_NAME% AppDirectory "%~dp0.."
IF DEFINED CONFIG_PATH (
    nssm set %SERVICE_NAME% AppEnvironmentExtra CMS_ENV=deploy
    nssm set %SERVICE_NAME% AppEnvironmentExtra CMS_CONFIG_PATH="%CONFIG_PATH%"
)

:: Set service to start automatically
nssm set %SERVICE_NAME% Start SERVICE_AUTO_START

:: Set recovery options
nssm set %SERVICE_NAME% AppRestartDelay 5000

:: Set service display name and description
nssm set %SERVICE_NAME% DisplayName "CMS Documentation Server"
nssm set %SERVICE_NAME% Description "CMS - Modern Documentation Platform Server"

:: Set service to run as a specific user (optional)
:: Uncomment and modify the following lines if you want to run as a specific user
:: nssm set %SERVICE_NAME% ObjectName "DOMAIN\username"
:: nssm set %SERVICE_NAME% ObjectPassword "password"

ECHO.
ECHO Service installed successfully!
ECHO.
ECHO To start the service, run:
ECHO   nssm start %SERVICE_NAME%
ECHO.
ECHO To check the service status, run:
ECHO   nssm status %SERVICE_NAME%
ECHO.
ECHO To view service logs, run:
ECHO   nssm logs %SERVICE_NAME%
ECHO.
PAUSE
