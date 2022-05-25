import os
import subprocess

##############
### CONFIG ###
##############

TARGET = "armv7-unknown-linux-gnueabihf"

#############
### SETUP ###
#############
is_ci = os.getenv("CI") is not None

# Load cross compile config. See `cross-compile-info.toml.template` for syntax.
cross_compile_info_file_name = "cross-compile-info.toml"
if is_ci:
    cross_compile_info_file_name = "cross-compile-info.ci.toml"

# These are necessary for some reason?
# RUSTFLAGS += "-Clink-args=-Xlinker -rpath=/usr/lib/arm-linux-gnueabihf " 

# TODO: Load from env
# dotenv.load_dotenv()
# os.environ["TARGET"] = 

###############
### RUNNING ###
###############
subprocess.run(f'cargo run -p across --release -- --config {cross_compile_info_file_name} --target {TARGET} --features use-openssl-vendored --release', check=True, shell=True)
