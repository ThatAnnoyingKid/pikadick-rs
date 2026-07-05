TARGET = aarch64-unknown-linux-gnu
PACKAGE = pikadick
DEBIAN_VERSION = 0.0.0
DEBIAN_REVISION = 1
DEBIAN_ARCH = arm64
DEB_NAME = ${PACKAGE}_${DEBIAN_VERSION}-${DEBIAN_REVISION}_${DEBIAN_ARCH}.deb
HOST = dagger-global

.PHONY: deploy

deploy:
	debian-sysroot-build --target ${TARGET} --package ${PACKAGE} --install-package libc6 --install-package libc6-dev --install-package linux-libc-dev --install-package libgcc-12-dev --install-package libopus-dev
	cargo deb --target ${TARGET} --no-build --no-strip
	deploy-deb target/${TARGET}/debian/${DEB_NAME} ${HOST}