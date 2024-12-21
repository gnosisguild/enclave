#!/usr/bin/env bash

docker build -t norepo/dummy:666 -f ./.deploy/dummy.Dockerfile .
