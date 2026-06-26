FROM gcr.io/oss-fuzz-base/base-builder-rust@sha256:fa90769bae4f3d60304954bc589671fa363fa62d49296489ce8180accca268f2

COPY . $SRC/openpolysphere
WORKDIR $SRC/openpolysphere
COPY .clusterfuzzlite/build.sh $SRC/build.sh
