#!/usr/bin/env python3
from chainbot import CLI
import json

a = CLI()
print("make config")
cfg=a.gen(2)
print(json.dumps(cfg, indent=4))
a.prepare_cfg(cfg)