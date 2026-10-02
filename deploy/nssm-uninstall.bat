@echo off
:: CMS Server - NSSM Uninstallation Script
:: This script removes the CMS Windows service
::
:: Usage:
::   nssm-uninstall.bat [service_name]

SETLOCAL ENABLEDELAYEDEXPANSION

:: Default service name
SET SERVICE_NAME=CMSServer

:: Parse command line arguments
IF NOT "%~1"=="" SET SERVICE_NAME=%~1

:: Check if NSSM is available
WHERE nssm >nul 2>&1
IF %ERRORLEVEL% NEQ 0 (
    ECHO ERROR: NSSM is not installed or not in PATH
    PAUSE
    EXIT /B 1
)

ECHO Stopping %SERVICE_NAME% service...
nssm stop %SERVICE_NAME% || (
    ECHO WARNING: Service may not be running
)

ECHO Removing %SERVICE_NAME% service...
nssm remove %SERVICE_NAME% confirm || (
    ECHO ERROR: Failed to remove service
    PAUSE
    EXIT /B 1
)

ECHO.
ECHO Service %SERVICE_NAME% removed successfully!
ECHO.
PAUSE
