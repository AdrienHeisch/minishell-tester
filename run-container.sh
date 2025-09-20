#!/bin/bash

# DO NOT USE --minishell TO SET THE PATH WITH THIS SCRIPT
# EDIT THIS LINE TO INCLUDE YOUR PROJECT IN THE CONTAINER
PROGRAM_PATH=../minishell/minishell

NAME=maxitest

if command -v podman &>/dev/null; then
  CONTAINER=podman
elif command -v docker &>/dev/null; then
  CONTAINER=docker
else
  echo "Docker or podman required."
  exit 1
fi

if [ $1 == "run" ]; then
  ARGS="-m /bin/minishell"
fi

$CONTAINER image build -f Containerfile -t $NAME . >/dev/null
$CONTAINER run \
  --rm \
  --tty \
  --pids-limit 4096 \
  -v "$PWD":"/usr/src/$NAME" \
  -v "$PWD/$PROGRAM_PATH":"/bin/minishell" \
  $NAME \
  /bin/maxitest $@ $ARGS
