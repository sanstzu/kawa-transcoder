FROM rust:alpine

ENV AWS_ACCESS_KEY_ID=$AWS_ACCESS_KEY_ID
ENV AWS_SECRET_ACCESS_KEY=$AWS_SECRET_ACCESS_KEY

RUN ["apk", "add" , "build-base", "protobuf", "protobuf-dev", "protoc", "ffmpeg", "aws-cli"]

WORKDIR /usr/src/app

COPY . .

RUN ["cargo", "build", "--release"]

RUN ["chmod", "+x", "entrypoint.sh"]

EXPOSE 50051

ENTRYPOINT ["./entrypoint.sh"]