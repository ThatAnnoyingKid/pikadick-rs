import argparse
import tomllib
import subprocess
import paramiko
import json
import os

def main():
	print('Loading config...')
	config = None
	with open("upload-test-config.toml", "rb") as file:
		config = tomllib.load(file)
	config_pi = config['pi']
	config_pi_target = config_pi['target']
		
	print('Compiling...')
	subprocess.run(f'cargo run -p across -- --cargo-build-wrapper "cargo test --no-run" --target {config_pi_target} --features wrapper', check=True)
	
	print('Parsing cargo metadata...')
	cargo_metadata_process = subprocess.run(f'cargo metadata --format-version 1', check=True, capture_output=True)
	cargo_metadata = json.loads(cargo_metadata_process.stdout)
	cargo_metadata_target_directory = cargo_metadata['target_directory']
	
	target_directory = os.path.join(cargo_metadata_target_directory, config_pi_target)
	target_profile_directory = os.path.join(target_directory, 'debug')
	examples_directory = os.path.join(target_profile_directory, 'examples')
	
	print('Uploading...')
	with paramiko.Transport((config_pi['address'], 22)) as transport:
		transport.connect(None, config_pi['user'], config_pi['password'])
		
		print('Opening SFTP...')
		with paramiko.SFTPClient.from_transport(transport) as sftp:
			print('Uploading `vcgencmd`...')
			vcgencmd_path = os.path.join(examples_directory, 'vcgencmd')
			sftp.put(vcgencmd_path, 'vcgencmd')
	
if __name__ == '__main__':
	main()