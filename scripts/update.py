import urllib.request
import os
import zipfile

ARTIFACT_URL = 'https://nightly.link/ThatAnnoyingKid/pikadick-rs/workflows/BuildRpi/master/pikadick.zip'
OUT_PATH = 'artifacts'
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


print('Extracting...')
zip = zipfile.ZipFile(ZIP_OUT_PATH)
zip.extractall(path = OUT_PATH)
zip.close()

print('Done.')