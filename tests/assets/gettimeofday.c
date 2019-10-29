#include <sys/types.h>
#include <unistd.h>
#include <sys/time.h>
#include <stdio.h>

int main(int argc, const char *argv[]) {
  FILE* fout = fopen("out.txt", "w");
  struct timeval tv;
  gettimeofday(&tv, NULL);

  struct timeval prev_tv = tv;
  sleep(1);
  gettimeofday(&tv, NULL);

  if (prev_tv.tv_sec == tv.tv_sec || prev_tv.tv_usec == tv.tv_usec) {
    fprintf(fout, "%d\n", 0);
  } else {
    fprintf(fout, "%d\n", 1);
  }

  fclose(fout);

	return 0;
}

