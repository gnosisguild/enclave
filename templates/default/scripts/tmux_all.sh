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

# Select the third pane and create its bottom counterpart
# tmux select-pane -t 5
# tmux split-window -v

# Reorganize layout to make it more even
tmux select-layout tiled

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

# Attach to the session
tmux attach-session -t "$SESSION_NAME"
