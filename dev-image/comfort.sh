#!/bin/bash
# Some nice things for developers to have

apt-get update -y && apt-get install -y neovim bash-completion htop less

echo "if [ -f /etc/bash_completion ] && ! shopt -oq posix; then" >> ~/.bashrc
echo "    . /etc/bash_completion" >> ~/.bashrc
echo "fi" >> ~/.bashrc
source ~/.bashrc
