import zipfile
import argparse
import sys
import os

parser = argparse.ArgumentParser(description='Package this program')
parser.add_argument('--target', action='store', help='the program target')
parser.add_argument('--release', action='store_true', help='release profile')
parser.add_argument('--exe-name', action='store', help='the exe name')
args = parser.parse_args()

profile = 'debug'
if args.release:
	profile = 'release'

base_path = f'target/{args.target}/{profile}' 
exe_path = f'{base_path}/{args.exe_name}'
package_path = f'{base_path}/{args.exe_name}.zip'

print('Packaging Options:')
print(f'Target: {args.target}')
print(f'Release: {args.release}')
print(f'Exe Name: {args.exe_name}')
print(f'Base Path: {base_path}')
print(f'Package Path: {package_path}')
print()

if args.target is None:
	sys.exit('missing --target')
	
if args.exe_name is None:
	sys.exit('missing --exe-name')

print(f'Creating package')
with zipfile.ZipFile(package_path, 'w') as package:
	print('Writing exe')
	package.write(exe_path, args.exe_name)
	
	print('Writing static folder')
	for path in os.listdir('static'):
		print(f'    Writing {path}')
		src_path = f'static/{path}'
		package.write(src_path, src_path)

