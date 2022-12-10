export DEPLOY_TARGET=
export RPI_DEPLOY = cargo run -p rpi-deploy --

.PHONY: pkg deploy

pkg: 
	$(RPI_DEPLOY) package
	
deploy:
	$(RPI_DEPLOY) deploy --name $(DEPLOY_TARGET)