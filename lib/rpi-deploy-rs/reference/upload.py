import paramiko
import scp
import dotenv
import os
import tqdm
import argparse
import sys

def ssh_exec(ssh_client, command):
    stdin, stdout, stderr = ssh_client.exec_command(command)
    stdout.channel.recv_exit_status()
    stdout_lines = stdout.readlines()
    stderr_lines = stderr.readlines()
    for line in stdout_lines:
        print(line)
    for line in stderr_lines:
        print(line)


parser = argparse.ArgumentParser(description='Upload this program')
parser.add_argument('--target', action='store', help='the program target')
parser.add_argument('--release', action='store_true', help='release profile')
parser.add_argument('--project-name', action='store', help='the project name')
args = parser.parse_args()

profile = 'debug'
if args.release:
	profile = 'release'

if args.target is None:
	sys.exit('target cannot be empty')

progress_dict = {}

def progress(filename, size, sent):
    if sent == 0:
        progress_dict[filename] = tqdm.tqdm(total=size, desc=str(filename), unit='megabyte', unit_scale=True)
    elif sent == size:
        progress_dict[filename].n = sent
        progress_dict[filename].update(0)

        progress_dict[filename].close()
        del progress_dict[filename]
    else:
        progress_dict[filename].n = sent
        progress_dict[filename].update(0)

env_config = dotenv.dotenv_values('.env')

ssh_address = env_config.get("SSH_ADDRESS")
ssh_port = 22
ssh_username = env_config.get("SSH_USERNAME")
ssh_password = env_config.get("SSH_PASSWORD")

if ssh_address is None:
	sys.exit('missing ssh address')
	
if ssh_username is None:
	sys.exit('missing ssh username')
	
if ssh_password is None:
	sys.exit('missing ssh password')

print(f'Logging in to ssh://{ssh_username}@{ssh_address}:{ssh_port}...')
with paramiko.SSHClient() as ssh_client:
    ssh_client.load_system_host_keys()
    ssh_client.connect(ssh_address, ssh_port, ssh_username, ssh_password)
    
    with scp.SCPClient(ssh_client.get_transport(), progress=progress) as scp_client: 
        print('Copying pkg...')
        version = '0.0.0'
        deb_arch = 'armhf'
        scp_client.put(f'target/{args.target}/debian/{args.project_name}_{version}_{deb_arch}.deb', f'{args.project_name}.deb')
    
    print('Installing...')
    ssh_exec(ssh_client, f'sudo dpkg -i --force-confold {args.project_name}.deb')
    
    print('Cleaning Up..')
    ssh_exec(ssh_client, f'rm {args.project_name}.deb')