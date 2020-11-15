import os
import subprocess

##############
### CONFIG ###
##############

# I'm putting this here since custom profiles are still nightly.
USE_LTO = True # Bool: True/False
NUM_CODEGEN_UNITS = 1 # 0 < n < 255
OPT_LEVEL = 3 # 0 <= n <= 3

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

RUSTFLAGS += "-Ccodegen-units={} ".format(NUM_CODEGEN_UNITS)
RUSTFLAGS += "-Copt-level={} ".format(OPT_LEVEL)

if USE_LTO:
    RUSTFLAGS += "-Clto -Cembed-bitcode=yes"
    # RUSTFLAGS += "-Clinker-plugin-lto " # This is used to make compilation emit llvm bitcode. 
                                          # The linker must be able to understand that.

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

subprocess.call(cmd_list, env=env)

if USE_STRIP:
    subprocess.call([ STRIP_BIN, 'target/{}/release/pikadick'.format(TARGET) ])

