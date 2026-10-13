.PHONY: zizmor web web-serve

zizmor:
	UV_NO_CACHE=true UV_ISOLATED=true uvx zizmor .

web:
	python3 tools/web.py build

web-serve:
	python3 tools/web.py serve
