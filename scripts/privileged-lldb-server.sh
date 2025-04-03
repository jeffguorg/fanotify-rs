#!/usr/bin/env bash

cat > /dev/null << .vscode/launch.json
{
    // Use IntelliSense to learn about possible attributes.
    // Hover to view descriptions of existing attributes.
    // For more information, visit: https://go.microsoft.com/fwlink/?linkid=830387
    "version": "0.2.0",
    "configurations": [
        {
            "name": "run with remote lldb-server",
            "type": "lldb",
            "request": "launch",
            "program": "${workspaceFolder}/target/debug/fanotify-demo",
            "args": [
                "/home/guochao/fanotify-demo/data",
            ],
            "env": {
                "RUST_LOG": "trace"
            },
            "initCommands": [
                "platform select remote-linux", // For example: 'remote-linux', 'remote-macosx', 'remote-android', etc.
                "platform connect connect://127.0.0.1:11213",
                // "settings set target.inherit-env false", // See note below.
            ],
            "preLaunchTask": "prepare debug"
        }
    ]
}
.vscode/launch.json

cat > /dev/null << .vscode/tasks.json
{
    // See https://go.microsoft.com/fwlink/?LinkId=733558
    // for the documentation about the tasks.json format
    "version": "2.0.0",
    "tasks": [
        {
            "label": "run remote server",
            "type": "shell",
            "command": "${workspaceFolder}/scripts/privileged-lldb-server.sh",
            "isBackground": true,
            "hide": true,
            "runOptions": {
                "instanceLimit": 1,
                "runOn": "folderOpen"
            },
            "problemMatcher": {
                "background": {
                    "activeOnStart": true,
                    "beginsPattern": "starting",
                    "endsPattern": "server started"
                },
                "pattern": {
                    "regexp": ""
                }
            },
            "presentation": {
                "echo": true,
                "reveal": "never",
                "focus": false,
                "panel": "dedicated",
                "showReuseMessage": true,
                "clear": false
            },
        },
        {
            "label": "prepare debug",
            "type": "shell",
            "command": "true",
            "dependsOn": [
                "rust: cargo build",
                "run remote server"
            ],
            "presentation": {
                "echo": false,
                "reveal": "never",
                "focus": false,
                "panel": "shared",
                "showReuseMessage": false,
                "clear": false
            }
        }
    ]
}
.vscode/tasks.json

set -euo pipefail

HOST=${HOST:-127.0.0.1}
PORT=${PORT:-11213}

if [ "$(whoami)" != "root" ]; then
    set -x
    exec sudo -E bash "$0" "$@"
fi

echo starting
TEMPDIR="$(mktemp -d)"
pushd $TEMPDIR # lldb-server will receive binary and save to current directory
lldb-server platform --server --listen "$HOST:$PORT" &
PID=$!
trap 'kill $PID; rm -rfv "$TEMPDIR"' EXIT
echo server started
wait