import os
import subprocess

# TODO: Run all this in a WSL shell on Windows. Look for native deps like perl.

##############
### CONFIG ###
##############

# Hardcode for now
TARGET = "armv7-unknown-linux-gnueabihf"

# Hardcode for now
LINKER = "arm-linux-gnueabihf-gcc"

# Runs 'strip' on the final binary
USE_STRIP = True

# Hardcode for now
STRIP_BIN = "arm-linux-gnueabihf-strip"

#################
### INTERNALS ###
#################

# Note: Ignores previously set RUSTFLAGS. Extend this script if you want more.
RUSTFLAGS = ""

#############
### SETUP ###
#############

RUSTFLAGS += "-Clinker={} ".format(LINKER)

# These are necessary for some reason?
# RUSTFLAGS += "-Clink-args=-Xlinker -rpath=/usr/lib/arm-linux-gnueabihf " 

# TODO: Load from env
# dotenv.load_dotenv()
# os.environ["TARGET"] = 

###############
### RUNNING ###
###############

cmd_list = [ "cargo", "build" ]
cmd_list.extend([ "--target", TARGET ])
cmd_list.extend([ "--features", "use-openssl-vendored" ])
cmd_list.extend([ "--release" ])

env = {}
env.update(os.environ)
env["RUSTFLAGS"] = RUSTFLAGS
env["RUST_BACKTRACE"] = "1"

subprocess.call(cmd_list, env=env)

if USE_STRIP:
    subprocess.call([ STRIP_BIN, 'target/{}/release/pikadick'.format(TARGET) ])

