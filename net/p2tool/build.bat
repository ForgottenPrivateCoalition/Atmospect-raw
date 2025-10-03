@echo off
:loop
cls
cargo build --release

pause

goto loop
