#!/bin/bash
# run_bot.sh – Watchdog script for Sharklet
# Auto-restarts the bot if it crashes

cd ~/sharklet || exit 1

while true; do
    echo "[watchdog] $(date) – Starting Sharklet..."
    cargo run --release 2>&1 | tee -a logs/sharklet.log
    echo "[watchdog] $(date) – Bot exited. Restarting in 5 seconds..."
    sleep 5
done