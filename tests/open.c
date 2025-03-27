// open.c
//
//   open [-f]
//
// Write to stderr (or stdout if stderr is closed) a message for each open
// file descriptor.  With the -f option, prints the flags as well.

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <fcntl.h>
#include <linux/limits.h>

int main (int argc, char *argv[])
{
    int foption = (argc > 1) && !strcmp (argv[1], "-f");
    int fdmax = sysconf (_SC_OPEN_MAX);
    FILE *fp = (fcntl (2, F_GETFL) < 0) ? stdout : stderr;
    int flags;

    for (int fd = 0;  fd < fdmax;  fd++) {
	if ((flags = fcntl (fd, F_GETFL)) >= 0) {
	    fprintf (fp, "File descriptor %d is open", fd);
	    if (foption && (flags & O_ACCMODE) == O_RDONLY)
		fputs (", RDONLY", fp);
	    if (foption && (flags & O_ACCMODE) == O_WRONLY)
		fputs (", WRONLY", fp);
	    if (foption && (flags & O_ACCMODE) == O_RDWR)
		fputs (", RDWR", fp);
	    if (foption && flags == 0)
		fputs (", but flags = 0", fp);
	    if (foption)
		fprintf (stderr, " (%d)", flags);
	    fputs ("\n", fp);
	}
    }

    return EXIT_SUCCESS;
}
