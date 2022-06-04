import urllib.request
import itertools
from collections import OrderedDict
import tarfile
import unix_ar
import os
import subprocess
import argparse

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
	output_file = f'src/{bindings_directory}/libbcm_host.rs'
	allowlist_bcm_host = ' '.join([
		'--allowlist-function bcm_host_.*',
		'--allowlist-function graphics_get_display_size',
		'--allowlist-var BCM_HOST_.*',
	])
	allowlist_vc_gencmd = ' '.join([
		'--allowlist-function vc_gencmd_.*', 
		'--allowlist-var GENCMDSERVICE_MSGFIFO_SIZE',
		
		# Replacement for `vc_gencmd_init`
		'--allowlist-function vc_vchi_gencmd_init',
	])
	blocklist_vc_gencmd = ' '.join([
		'--blocklist-function vc_gencmd_inum', 
		'--blocklist-function vc_gencmd_read_response_partial',
		'--blocklist-function vc_gencmd_close_response_partial', 
		'--blocklist-function vc_gencmd_read_partial_state',
		
		# This function unconditonally aborts, use `vc_vchi_gencmd_init` instead.
		'--blocklist-function vc_gencmd_init',
	])
	allowlist_vcos = '--allowlist-function vcos_.*'
	blocklist_vcos = ' '.join([
		'--blocklist-function vcos_pthreads_timer_reset', 
		'--blocklist-function vcos_kmalloc', 
		'--blocklist-function vcos_kcalloc',
		'--blocklist-function vcos_kfree', 
		'--blocklist-function vcos_log_set_level_all', 
		'--blocklist-function vcos_log_assert_cmd', 
		'--blocklist-function vcos_log_set_cmd', 
		'--blocklist-function vcos_log_status_cmd', 
		'--blocklist-function vcos_log_test_cmd', 
		
		# TODO: These should go in another section
		'--blocklist-function vc_dispman_init', 
		'--blocklist-function vc_dispmanx_resource_write_data_handle',
	])
	allowlist_vchi = '--allowlist-function vchi_.*'
	blocklist_vchi = ' '.join([
		'--blocklist-function vchi_crc_control', 
		'--blocklist-function vchi_allocate_buffer', 
		'--blocklist-function vchi_free_buffer', 
		'--blocklist-function vchi_current_time', 
		'--blocklist-function vchi_get_peer_version', 
		'--blocklist-function vchi_msg_queuev_ex', 
		'--blocklist-function vchi_msg_look_ahead', 
		'--blocklist-function vchi_held_msg_ptr', 
		'--blocklist-function vchi_held_msg_size', 
		'--blocklist-function vchi_held_msg_tx_timestamp', 
		'--blocklist-function vchi_held_msg_rx_timestamp', 
		'--blocklist-function vchi_msg_iter_has_next', 
		'--blocklist-function vchi_msg_iter_next', 
		'--blocklist-function vchi_msg_iter_remove', 
		'--blocklist-function vchi_msg_iter_hold',
		'--blocklist-function vchi_msg_iter_hold_next',
		'--blocklist-function vchi_bulk_queue_receive_reloc',
		'--blocklist-function vchi_bulk_queue_receive_reloc_func',
		'--blocklist-function vchi_bulk_queue_transmit_reloc',
	])
	
	subprocess.run(f'bindgen bindgen-bcm_host.h -o {output_file} {allowlist_bcm_host} {allowlist_vc_gencmd} {blocklist_vc_gencmd} {allowlist_vcos} {blocklist_vcos} {allowlist_vchi} {blocklist_vchi} --dynamic-loading libbcm_host --dynamic-link-require-all -- --target={clang_target} --sysroot=bundled/{arch} -Ibundled/{arch}/usr/include', check=True)

def main():
	parser = argparse.ArgumentParser(description='Rebuild bindings')
	parser.add_argument('--skip-apt', action='store_true', help='skip downloading apt packages')
	args = parser.parse_args()
	
	if not args.skip_apt:
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