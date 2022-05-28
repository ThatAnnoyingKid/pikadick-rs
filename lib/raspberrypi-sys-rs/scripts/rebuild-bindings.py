import urllib.request
import itertools
from collections import OrderedDict
import tarfile
import unix_ar
import os
import subprocess

HOST = "https://archive.raspberrypi.org"
DIST = "bullseye"
COMP = "main"
# ARCH = "arm64" # One of ['arm64', 'armhf', 'amd64', 'i386']

class Package:
	def __init__(self, name, version, description, filename):
		self.name = name
		self.version = version
		self.description = description
		self.filename = filename
		
	def __repr__(self):
		description = '\n\t\t'.join(self.description.split('\n'))
		return f'Package(\n\tname = {self.name},\n\tversion = {self.version},\n\tdescription = {description},\n\tfilename = {self.filename}\n)'
		
def parse_package(string):
	kv = OrderedDict()
	iter = (pair.split(': ') for pair in string.split('\n'))
	for pair in iter:
		if pair[0].startswith(' '):
			pair[0] = pair[0][1:]
			k, v = kv.popitem()
			kv[k] = v + '\n' + ': '.join(pair)
		else:
			kv[pair[0]] = pair[1]
		
	return Package(
		name = kv['Package'], 
		version = kv['Version'], 
		description = kv['Description'], 
		filename = kv['Filename']
	)
	
class AptPackageDownloader:
	def __init__(self):
		self.package_lists = {}
	
	def get_package_list(self, arch):
		maybe_package_list = self.package_lists.get(arch)
		if maybe_package_list is not None:
			return maybe_package_list
		
		packagesUrl = f'{HOST}/debian/dists/{DIST}/{COMP}/binary-{arch}/Packages'
		packages = None
		with urllib.request.urlopen(packagesUrl) as packagesRequest:
			packages = packagesRequest.read().decode('utf-8')
		parsed_packages = list(map(parse_package, filter(len, packages.split('\n\n'))))
		
		self.package_lists[arch] = parsed_packages
		
		return parsed_packages
	
	def download_package(self, arch, package_name, file_path):
		package_list = self.get_package_list(arch)
		
		target_package = next(package for package in package_list if package.name == package_name)
		package_url = f'{HOST}/debian/{target_package.filename}'
		
		with open(file_path, 'wb') as package_file:
			with urllib.request.urlopen(package_url) as package_request:
				package_file.write(package_request.read())

def deb_extract_file(deb_path, extract_path):
	ar_file = unix_ar.open(deb_path, 'r')
	ar_file_info_list = ar_file.infolist()
	tarball_name = next(filter(lambda name: name.startswith('data.'), map(lambda info: info.name.decode('utf-8'), ar_file_info_list)))
	tarball = ar_file.open(tarball_name)
	with tarfile.open(fileobj=tarball) as tar_file:
		extract_file = tar_file.extractall(extract_path)
	ar_file.close()
	
def install_deb(apt_package_downloader, arch, package_name, extract_path):
	package_path = os.path.join(extract_path, f'{package_name}.deb')
	apt_package_downloader.download_package(arch, package_name, package_path)
	deb_extract_file(package_path, extract_path)
	
def generate_bindings(arch):
	clang_target = None
	bindings_directory = None
	# TODO: Is musl support an option?
	if arch == 'arm' or arch == 'armhf':
		clang_target = 'arm-linux-gnueabihf'
		bindings_directory = 'arm_bindings'
	elif arch == 'aarch64' or arch == 'arm64':
		clang_target = 'aarch64-linux-gnueabihf'
		bindings_directory = 'arm64_bindings'
	else:
		raise Exception(f'unsupported arch `{arch}`')
	
	# libbcm_host
	subprocess.run(f'bindgen bundled/{arch}/usr/include/bcm_host.h -o src/{bindings_directory}/libbcm_host.rs --allowlist-function bcm_host_.* --allowlist-function graphics_get_display_size --allowlist-var BCM_HOST_.* --dynamic-loading libbcm_host --dynamic-link-require-all -- --target={clang_target} --sysroot=bundled/{arch} -Ibundled/{arch}/usr/include', check=True)

	# libvcos
	subprocess.run(f'bindgen bundled/{arch}/usr/include/interface/vcos/vcos.h -o src/{bindings_directory}/libvcos.rs --allowlist-function vcos_.* --dynamic-loading libvcos --dynamic-link-require-all -- --target={clang_target} --sysroot=bundled/{arch} -Ibundled/{arch}/usr/include', check=True)

def main():
	apt_package_downloader = AptPackageDownloader()
	
	armhf_dir = "bundled/armhf"
	arm64_dir = "bundled/arm64"
	
	print('Getting packages list...')
	package_list_armhf = apt_package_downloader.get_package_list('armhf')
	print(f'Got {len(package_list_armhf)} armhf packages')
	package_list_arm64 = apt_package_downloader.get_package_list('arm64')
	print(f'Got {len(package_list_arm64)} arm64 packages')
	
	try:
		os.mkdir(armhf_dir)
	except FileExistsError:
		pass
	try:
		os.mkdir(arm64_dir)
	except FileExistsError:
		pass
		
	needed_packages = [
		"libraspberrypi-dev",
		"libc6-dev",
		"linux-libc-dev",
	]
	
	for package in needed_packages:
		print(f'Installing `{package}` (armhf)...')
		install_deb(apt_package_downloader, 'armhf', package, armhf_dir)
		print(f'Installing `{package}` (arm64)...')
		install_deb(apt_package_downloader, 'arm64', package, arm64_dir)
		
	print('Generating bindings (armhf)...')
	generate_bindings("armhf")
	print('Generating bindings (arm64)...')
	generate_bindings("arm64")
	
	print('Done')

if __name__ == "__main__":
	main()