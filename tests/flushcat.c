// flushcat.c
//
//   Copy stdin to stdout using unbuffered writes and appending $ to each line

#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>

int main (int argc, char *argv[])
{
    int c;

    while ((c = getchar()) != EOF) {
	if (c == '\n')
	  write (1, "$", 1);
	write (1, &c, 1);
    }

    exit (EXIT_SUCCESS);
}
