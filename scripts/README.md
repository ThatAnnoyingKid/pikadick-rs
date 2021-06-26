# Scripts
A collection of helpful scripts for development, deployment, and CI. 
They should be in python3 to maintain cross-platform compatibility. 
As such, python3 is needed to run these. 
External dependencies should only be added in EXTREME circumstances.

# cross.py
A tool to aid in cross compilation.

## Dependencies

### tomli
This dependency was introduced to parse toml config files. 
This is added to try to provide configs in toml only.
The pip `toml` package is far less maintained than this one.
Use `pip install tomli` to install.

# update.py
A tool to graph the latest rpi artifact from github and put it in `artifacts/pikadick`.