// sleeper.c
//
//   sleeper FIRST DELAY SECOND [STATUS]
//
// Copy FIRST to stdout & flush stdout unless FIRST = -, sleep DELAY seconds,
// and copy SECOND to stdout.  Exit with status STATUS (default EXIT_SUCCESS).

#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

void Sleep (double time)
{
    struct timespec req, rem;

    req.tv_sec  = time;
    req.tv_nsec = (time - req.tv_sec) * 1000000000L;

    while (nanosleep (&req, &rem) < 0) {
	req.tv_sec  = rem.tv_sec;
	req.tv_nsec = rem.tv_nsec;
    }
}


int main (int argc, char *argv[])
{
    if (argc < 4 || 5 < argc) {
	fprintf (stderr, "sleeper FIRST DELAY SECOND [STATUS]\n");
	exit (EXIT_FAILURE);
    }

    if (strcmp (argv[1], "-")) {
	printf ("%s\n", argv[1]);
	fflush (stdout);
    }

    Sleep (atof(argv[2]));

    printf ("%s\n", argv[3]);

    exit (argv[4] ? atof(argv[4]) : EXIT_SUCCESS);
}
