.PHONY: build-cli
build-cli:
	cargo build -p anchor-cli --release
	cp target/release/anchor cli/npm-package/anchor

.PHONY: clean
clean:
	find . -type d -name .anchor -print0 | xargs -0 rm -rf
	find . -type d -name node_modules -print0 | xargs -0 rm -rf
	find . -type d -name target -print0 | xargs -0 rm -rf

.PHONY: publish
publish:
	cd lang/syn/ && cargo publish && cd ../../
	sleep 25
	cd lang/derive/accounts/ && cargo publish && cd ../../../
	sleep 25
	cd lang/attribute/access-control/ && cargo publish && cd ../../../
	sleep 25
	cd lang/attribute/account/ && cargo publish && cd ../../../
	sleep 25
	cd lang/attribute/constant/ && cargo publish && cd ../../../
	sleep 25
	cd lang/attribute/error/ && cargo publish && cd ../../../
	sleep 25
	cd lang/attribute/interface/ && cargo publish && cd ../../../
	sleep 25
	cd lang/attribute/program/ && cargo publish && cd ../../..
	sleep 25
	cd lang/attribute/state/ && cargo publish && cd ../../../
	sleep 25
	cd lang/attribute/event/ && cargo publish && cd ../../../
	sleep 25
	cd lang/ && cargo publish && cd ../
	sleep 25
	cd spl/ && cargo publish && cd ../
	sleep 25
	cd client/ && cargo publish && cd ../
	sleep 25
	cd cli/ && cargo publish && cd ../
	sleep 25
