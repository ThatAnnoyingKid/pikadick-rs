export DEPLOY_TARGET=

.PHONY: pkg deploy

pkg: 
	cargo run -p rpi-deploy package
	
deploy:
	cargo run -p rpi-deploy deploy --name $(DEPLOY_TARGET)