CC=gcc
CFLAGS=-std=c11 -Wall -pedantic -no-pie -I.
NAME=Bash

%.o: %.c
	$(CC) -c -o $@ $< $(CFLAGS)

$(NAME): process.o main.o parse.o
	$(CC) -o $@ $^ $(CFLAGS)

.PHONY: all
all: $(NAME)

.PHONY: rust
rust: main.o parse.o ffi.o
	cargo build --lib
	rm -f ./libprocess.a
	mv -f ./target/debug/libprocess.a ./
	$(CC) -o $(NAME) main.o parse.o libprocess.a ffi.o -pthread -ldl $(CFLAGS)

.PHONY: clean
clean:
	rm -f process.o main.o $(NAME)
	rm -f ffi.o libprocess.a
	rm -rf ./target
