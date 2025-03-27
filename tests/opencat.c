// opencat.c
//
// Write to stdout a message for each open file descriptor and then copy
// stdin to stdout.

#include <stdio.h>
#include <stdlib.h>
#include <fcntl.h>
#include <linux/limits.h>

int main (int argc, char *argv[])
{
    int fd;

    for (fd = 0;  fd < NR_OPEN;  fd++) {
	if (fcntl (fd, F_GETFL) >= 0)
	    fprintf (stdout, "File descriptor %d is open\n", fd);
    }

    int c;
    while ((c = getchar()) != EOF)
	putchar(c);

    return EXIT_SUCCESS;
}
