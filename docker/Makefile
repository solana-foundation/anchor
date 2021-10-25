WORKDIR=$(PWD)
#
# Extract anchor version from the Cargo.toml.
#
ANCHOR_CLI=v$(shell awk -F ' = ' '$$1 ~ /version/ { gsub(/[\"]/, "", $$2); printf("%s",$$2) }' ../cli/Cargo.toml)
#
# Solana toolchain.
#
SOLANA_CLI=v1.8.0
#
# Build version should match the Anchor cli version.
#
IMG_ORG ?= projectserum
IMG_VER ?= $(ANCHOR_CLI)

.PHONY: build build-push build-shell publish

default:

build: build/Dockerfile
	@docker build \
	--build-arg ANCHOR_CLI=$(ANCHOR_CLI) \
	--build-arg SOLANA_CLI=$(SOLANA_CLI) \
	$@ -t $(IMG_ORG)/$@:$(IMG_VER)

build-push:
	@docker push $(IMG_ORG)/build:$(IMG_VER)

build-shell:
	@docker run -ti --rm --net=host \
		-v $(WORKDIR)/..:/workdir \
		$(IMG_ORG)/build:$(IMG_VER) bash

publish: build build-push
