centos-builder:
	docker build -t centos-rust-builder -f BuilderImages/centos7.Dockerfile .

binaries:
	mkdir -p target/centos/
	docker run --rm \
		--name centos-pink-lady-builder \
		-v ${PWD}/app:/app \
		-v ${PWD}/target/centos:/app/target/release \
		centos-rust-builder \
		/root/.cargo/bin/cargo build --release
