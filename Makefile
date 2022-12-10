export DEPLOY_TARGET=
export RPI_DEPLOY = cargo run -p rpi-deploy --

.PHONY: pkg pkg-ci deploy

pkg: 
	$(RPI_DEPLOY) package
	
pkg-ci:
	$(RPI_DEPLOY) package --config cross-compile-info.ci.toml
	
deploy:
	$(RPI_DEPLOY) deploy --name $(DEPLOY_TARGET)