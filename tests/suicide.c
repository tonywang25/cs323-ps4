// suicide.c
//
//   suicide N Message
//
// Sleep N seconds, try to kill process group with SIGINT, and then print
// message if still alive.  N can be a floating point number.

#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <signal.h>
#include <sys/types.h>
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
    Sleep (atof (argv[1]));
    killpg (0, SIGINT);
    fprintf (stderr, "suicide: %s\n", argv[2]);
    return EXIT_SUCCESS;
}
