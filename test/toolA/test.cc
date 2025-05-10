#include "libA/subFolder/other.h"
#include <iostream>

extern "C" {
extern int fa(double);
extern int fb();
}

int main() {
  std::cout<<fa(1.2)+fb()+fa2()<<"\n";
  return 0;
}
