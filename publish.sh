#!/usr/bin/env sh

nix build --print-out-paths .#armv7-gnu | cachix push upgradegamma

ssh root@192.168.1.153 pkill standing

scp result/bin/standing_bot root@192.168.1.153:/data/media/0/standing/

ssh root@192.168.1.153 -p 8022 'cd /data/media/0/standing/ && ./standing_bot.sh'
