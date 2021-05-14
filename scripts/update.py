#!/usr/bin/env python3

import urllib.request
import os
import zipfile
import stat

ARTIFACT_URL = 'https://nightly.link/ThatAnnoyingKid/pikadick-rs/workflows/BuildRpi/master/pikadick.zip'
OUT_PATH = 'artifacts'
OUT_EXE_PATH = OUT_PATH + '/pikadick'
ZIP_OUT_PATH = OUT_PATH + '/pikadick.zip'

print('Downloading...')
response = urllib.request.urlopen(ARTIFACT_URL)

# Failures usually mean that it already exists
try:
    os.mkdir(OUT_PATH)
except FileExistsError as error:
    pass
    

file = open(ZIP_OUT_PATH, "wb")
file.write(response.read())

print('Deleting old executable...')
# Failures usually mean that it is not present
try:
    os.remove(OUT_EXE_PATH)
except FileNotFoundError as error:
    pass

print('Extracting...')
zip = zipfile.ZipFile(ZIP_OUT_PATH)
zip.extractall(path = OUT_PATH)
zip.close()

print('Making executable...');
os.chmod(OUT_EXE_PATH, os.stat(OUT_EXE_PATH).st_mode | stat.S_IEXEC)

print('Done.')