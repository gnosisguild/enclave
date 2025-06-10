#!/usr/bin/env bash

# Check if tmux is installed
if ! command -v tmux &> /dev/null; then
    echo "ERROR: tmux is not installed or not in PATH"
    echo "Please install tmux first:"
    echo "  - Ubuntu/Debian: sudo apt install tmux"
    echo "  - macOS: brew install tmux"
    echo "  - CentOS/RHEL: sudo yum install tmux"
    exit 1
fi

# Create a new tmux session with 6 splits (2 rows of 3) and run specific commands
SESSION_NAME="enclave-splits"

# Check if we're already inside a tmux session
if [ -n "$TMUX" ]; then
    echo "Already inside a tmux session. Creating splits in current session..."
    # We're inside tmux, so just create the splits in the current session
else
    echo "Not in tmux. Creating new session '$SESSION_NAME'..."
    # Create new session
    tmux new-session -d -s "$SESSION_NAME"
fi

# Split into 3 vertical panes (top row)
tmux split-window -h
tmux split-window -h

# Select the first pane and create bottom row
tmux select-pane -t 0
tmux split-window -v

# Select the second pane and create its bottom counterpart
tmux select-pane -t 2
tmux split-window -v

# Select the third pane and create its bottom counterpart
tmux select-pane -t 4
tmux split-window -v

# Reorganize layout to make it more even
tmux select-layout tiled

sleep 2

# Run commands in each pane
tmux send-keys -t 1 'pnpm dev:evm' C-m
sleep 1

tmux send-keys -t 2 'pnpm dev:ciphernodes' C-m
sleep 1

tmux send-keys -t 3 'pnpm dev:server' C-m
sleep 1

tmux send-keys -t 4 'enclave program start' C-m
sleep 1

tmux send-keys -t 5 'pnpm dev:frontend' C-m

# Select the first pane to start
tmux select-pane -t 1

# Attach to the session only if we weren't already in tmux
if [ -z "$TMUX" ]; then
    tmux attach-session -t "$SESSION_NAME"
else
    echo "6-split layout created in current session with commands running!"
fi
