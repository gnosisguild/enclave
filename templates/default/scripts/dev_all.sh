#!/usr/bin/env bash

SESSION_NAME="enclave-splits"

# Check if tmux is available
if command -v tmux &> /dev/null; then
    echo "tmux found - using tmux session..."
    
    # Kill existing session if it exists
    if tmux has-session -t "$SESSION_NAME" 2>/dev/null; then
        echo "Killing existing session '$SESSION_NAME'..."
        tmux kill-session -t "$SESSION_NAME"
    fi
    
    echo "Creating new session '$SESSION_NAME'..."
    # Create new session
    tmux new-session -d -s "$SESSION_NAME"
    
    # Split into 3 vertical panes (top row)
    tmux split-window -h
    tmux split-window -h
    
    # Select the first pane and create bottom row
    tmux select-pane -t 1
    tmux split-window -v
    
    # Select the second pane and create its bottom counterpart
    tmux select-pane -t 3
    tmux split-window -v
    
    # Reorganize layout to make it more even
    tmux select-layout tiled
    
    # Run commands in each pane
    tmux send-keys -t 1 'pnpm dev:evm' C-m
    sleep 1
    tmux send-keys -t 2 'pnpm dev:ciphernodes' C-m
    sleep 1
    tmux send-keys -t 3 'TEST_MODE=1 pnpm dev:server' C-m
    sleep 1
    tmux send-keys -t 4 'enclave program start' C-m
    sleep 1
    tmux send-keys -t 5 'pnpm dev:frontend' C-m
    
    # Select the first pane to start
    tmux select-pane -t 1
    
    # Attach to the session
    tmux attach-session -t "$SESSION_NAME"
    
else
    echo "tmux not found - using pnpm concurrently..."
    
    # Check if pnpm is available
    if ! command -v pnpm &> /dev/null; then
        echo "ERROR: pnpm is not installed or not in PATH"
        echo "Please install pnpm or tmux to run this script"
        exit 1
    fi
    
    # Run all processes concurrently using pnpm
    pnpm concurrently \
        --names "EVM,CIPHER,SERVER,ENCLAVE,FRONTEND" \
        --prefix-colors "cyan,magenta,yellow,green,blue" \
        --kill-others-on-fail \
        "pnpm dev:evm" \
        "pnpm dev:ciphernodes" \
        "TEST_MODE=1 pnpm dev:server" \
        "enclave program start" \
        "pnpm dev:frontend"
fi
