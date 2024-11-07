
.PHONY: ensure build dev run clean lint benchmark test publish

PROJ=r008_quake2

RAIBUILD=$(PWD)/vendor/raibuild
CPRINT=$(RAIBUILD)/cprint.ts


# --------------------------------------------------------------------------- #
# ensure
# --------------------------------------------------------------------------- #

.PHONY: ensure
ensure:	
	rustup target add wasm32-unknown-unknown
	which wasm-bindgen || cargo install wasm-bindgen-cli && \
		cargo update -p wasm-bindgen --precise 0.2.95
	which mprocs || cargo install mprocs
	mkdir -p vendor
	-rm -rf $(PWD)/vendor/raibuild
	-ln -s $(MONOREPO_ROOT)/lib/raibuild $(PWD)/vendor/raibuild
	npm install
	make ensure-data

.PHONY: ensure-data
ensure-data:
	mkdir -p assets
	[ -f assets/q2dm1.bsp ] || cp $(MONOREPO_ROOT)/storage/raiment-studios-private/q2dm1.bsp assets

# --------------------------------------------------------------------------- #
# build
# --------------------------------------------------------------------------- #

.PHONY: build
build: ensure
	rm -rf dist && mkdir -p dist
	cargo build --release --target wasm32-unknown-unknown
	wasm-bindgen \
		--out-dir target \
		--target web target/wasm32-unknown-unknown/release/$(PROJ).wasm
	cp src/index.html dist/
	mkdir -p dist/assets && cp -R assets/ dist/assets/
	cp target/$(PROJ).js dist/
	cp target/$(PROJ)_bg.wasm dist/
	echo "$(shell DATE)" > dist/build-timestamp.txt


# --------------------------------------------------------------------------- #
# dev
# --------------------------------------------------------------------------- #

.PHONY: dev dev-watch
dev-watch:
	npx nodemon \
		--watch src --watch assets \
		--ext rs,html,css,js,png,jpg,otf,blend \
		--exec "make build || exit 1" \

dev: ensure
	$(RAIBUILD)/make_multiple.ts dev-watch run-server

# --------------------------------------------------------------------------- #
# run
# --------------------------------------------------------------------------- #

.PHONY: run run-server
run: build	
	$(MAKE) run-server

run-server:
	npx serve --cors --listen 8099 dist


# --------------------------------------------------------------------------- #
# publish
# --------------------------------------------------------------------------- #

.PHONY: publish publish-source publish-deploy

publish-source:
	@$(CPRINT) "Building source to ensure any vendor dependencies are included"
	make build
	@$(CPRINT) "Ensuring remote repo is created"
	-gh repo create raiment-studios/$(PROJ) --public
	@$(CPRINT) "Cloning local & remote source to __temp1 and __temp2"
	rm -rf __temp1 __temp2 .git
	cp -aLR . __temp1
	rm -f __temp1/vendor/.gitignore
	git clone git@github.com:raiment-studios/$(PROJ).git __temp2
	@$(CPRINT) "Moving remote .git to local copy"
	mv __temp2/.git __temp1/.git
	rm -rf __temp2	
	@$(CPRINT) "Committing & pushing to remote"
	cd __temp1 && \
		git config user.email ridley.grenwood.winters@gmail.com && \
		git config user.name "Ridley Winters" && \
		git add . && \
		git commit -m "Automated commit from monorepo" && \
		git push 
	@$(CPRINT) "Cleaning up"
	rm -rf __temp1 __temp2 .git

publish: build publish-source publish-deploy

publish-deploy:
	@echo "Publishing..."
	deno install -Arf --global jsr:@deno/deployctl
	asdf reshim deno
	cd dist && deployctl \
		deploy --project=$(PROJ) --prod \
		https://jsr.io/@std/http/1.0.7/file_server.ts

# --------------------------------------------------------------------------- #
# clean
# --------------------------------------------------------------------------- #

.PHONY: clean
clean:
	git clean -Xdf