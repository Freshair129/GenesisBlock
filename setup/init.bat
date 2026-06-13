@echo off
setlocal
echo Initializing GoVibe Agent Infrastructure...
mkdir .agents\devops\handoff 2>nul
if not exist "AGENT.md" copy setup\AGENT.md AGENT.md
echo Setup Complete.
