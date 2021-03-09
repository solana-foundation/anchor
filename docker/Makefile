IMG_ORG ?= projectserum
IMG_VER ?= latest

WORKDIR=$(PWD)

.PHONY: build build-push build-shell

default:

build: build/Dockerfile
	@docker build $@ -t $(IMG_ORG)/$@:$(IMG_VER)

build-push:
	@docker push $(IMG_ORG)/anchorbuild:$(IMG_VER)

build-shell:
	@docker run -ti --rm --net=host \
		-v $(WORKDIR)/..:/workdir \
		$(IMG_ORG)/build:$(IMG_VER) bash
