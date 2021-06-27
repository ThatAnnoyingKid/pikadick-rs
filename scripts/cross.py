import os
import subprocess
import tomli

##############
### CONFIG ###
##############

# Hardcode for now
TARGET = "armv7-unknown-linux-gnueabihf"

# Run 'strip' on the final binary
USE_STRIP = True

#################
### INTERNALS ###
#################

# Note: Ignores previously set RUSTFLAGS. Extend this script if you want more.
RUSTFLAGS = ""

#############
### SETUP ###
#############
is_ci = os.getenv("CI") is not None

# Load cross compile config. See `cross-compile-info.toml.template` for syntax.
cross_compile_info_file_name = "cross-compile-info.toml"
if is_ci:
    cross_compile_info_file_name = "cross-compile-info.ci.toml."
cross_compile_info_file_name = os.path.join(os.getcwd(), cross_compile_info_file_name)
print("Parsing `{}`".format(cross_compile_info_file_name))
cross_compile_info_file = open(cross_compile_info_file_name, encoding="utf-8")
cross_compile_info = tomli.load(cross_compile_info_file)

linker = cross_compile_info[TARGET]['linker']
strip_bin = cross_compile_info[TARGET]['strip']
RUSTFLAGS += "-Clinker={} ".format(linker)

# These are necessary for some reason?
# RUSTFLAGS += "-Clink-args=-Xlinker -rpath=/usr/lib/arm-linux-gnueabihf " 

# TODO: Load from env
# dotenv.load_dotenv()
# os.environ["TARGET"] = 

###############
### RUNNING ###
###############
print("Compiling")
cmd_list = [ "cargo", "build" ]
cmd_list.extend([ "--target", TARGET ])
cmd_list.extend([ "--features", "use-openssl-vendored" ])
cmd_list.extend([ "--release" ])

env = {}
env.update(os.environ)
env.update(cross_compile_info[TARGET]['env'])
env["RUSTFLAGS"] = RUSTFLAGS
env["RUST_BACKTRACE"] = "1"

subprocess.call(cmd_list, env=env)

print("Stripping")
if USE_STRIP:
    subprocess.call([ strip_bin, 'target/{}/release/pikadick'.format(TARGET) ])
