@echo off
cd /d "%~dp0"
echo Строю мир (около 15 секунд), потом откроется окно игры.
echo Закрыть игру: закройте окно игры.
cargo run --release --offline --features native --bin play -- --native %*
pause
