#!/usr/bin/env bash

if [ ! -f .env ]; then
    cp .env.example .env
fi

cargo check
