#!/bin/sh
set -ex

cp doc/head.html doc/docs.html
target/release/vemf doc/gen.vemf < doc/raw.txt >> doc/docs.html