SHELL := /bin/sh

DOCS_DIR      := docs
DOCS_BUILD_DIR:= target/docs

DOC_MAIN      := $(DOCS_DIR)/main.tex
DOC_NAME      := $(basename $(notdir $(DOC_MAIN)))
DOC_CFG       := $(DOCS_DIR)/site.cfg
DOC_CSS       := $(DOCS_DIR)/site.css

PDF_DIR       := $(DOCS_BUILD_DIR)/pdf
HTML_DIR      := $(DOCS_BUILD_DIR)/html
HTML_TEMP_DIR := $(DOCS_BUILD_DIR)/make4ht

PDF_OUTPUT    := $(PDF_DIR)/$(DOC_NAME).pdf
HTML_OUTPUT   := $(HTML_DIR)/$(DOC_NAME).html
CSS_OUTPUT    := $(HTML_DIR)/site.css
.PHONY: all docs pdf html clean switch

define XPU_BLOCK
[tool.uv.sources]
torch = { index = "pytorch-xpu" }

[[tool.uv.index]]
name = "pytorch-xpu"
url = "https://download.pytorch.org/whl/xpu"
endef
export XPU_BLOCK


all: docs lib uv
lib:
	@cargo b --release
uv:
	@uv sync && maturin develop
docs:
	@rm -rf "$(DOCS_BUILD_DIR)"
	@$(MAKE) pdf html
pdf:
	@printf 'Building PDF documentation...\n'
	@mkdir -p "$(abspath $(PDF_DIR))"
	@cd "$(abspath $(DOCS_DIR))" && latexmk -pdf -interaction=nonstopmode -halt-on-error -file-line-error -outdir="$(abspath $(PDF_DIR))" "$(notdir $(DOC_MAIN))"
html:
	@printf 'Building HTML documentation...\n'
	@mkdir -p "$(abspath $(HTML_DIR))" "$(abspath $(HTML_TEMP_DIR))"
	@cd "$(DOCS_DIR)" && find . -type d | while IFS= read -r dir; do mkdir -p "$(abspath $(HTML_TEMP_DIR))/$$dir"; done
	@cd "$(abspath $(DOCS_DIR))" && make4ht -x -u -c "$(notdir $(DOC_CFG))" -f html5 -B "$(abspath $(HTML_TEMP_DIR))" -d "$(abspath $(HTML_DIR))" "$(notdir $(DOC_MAIN))" "fancylogo,mathml,2,fulltoc,next"
	@cp "$(DOC_CSS)" "$(CSS_OUTPUT)"
clean:
	@cargo clean
switch:
	@perl -0pi \
		-e '$$b = $$ENV{XPU_BLOCK};' \
		-e 'if (/"torch==\d+(?:\.\d+)*\+xpu"/) {' \
		-e '  s/("torch==\d+(?:\.\d+)*)\+xpu"/$$1"/;' \
		-e '  s/\n*\Q$$b\E\n*/\n/;' \
		-e '  print STDOUT "XPU disabled\n";' \
		-e '} else {' \
		-e '  s/("torch==\d+(?:\.\d+)*)"/$$1+xpu"/;' \
		-e '  s/\s*\z/\n\n$$b\n/;' \
		-e '  print STDOUT "XPU enabled\n";' \
		-e '}' pyproject.toml
defconfig:
	@cp cfg/rrr/def.toml rrr.toml
	@cp cfg/cc/curriculum_det.json curriculum.json
tarcode:
	@git ls-files -z -c -o --exclude-standard | tar --null -T - -czf ../code.tar.gz
tarmodel:
	tar -czf ../models.tar.gz rrr.toml models/ curriculum.json logs/
