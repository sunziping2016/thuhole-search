#include <cassert>
#include <codecvt>
#include <cstdio>
#include <cstdlib>
#include <locale>
#include <string>
#include <vector>

using namespace std;

struct Entry {
  int base;
  int check;
};

Entry *dat;
size_t dat_size;
u32string result;
vector<vector<int>> children;
vector<int> progress;
wstring_convert<codecvt_utf8<char32_t>, char32_t> convert;
int count = 0;

void find(int index) {
  int base = dat[index].base;
  if (result.size() >= 2 && result[result.size() - 2] == ' ') {
    auto s = convert.to_bytes(result.data(), result.data() + result.size());
    printf("%s\t%d\n", s.c_str(), base);
    count = (count + 1) % 1000;
    if (count == 0) {
      double p = 0.0;
      double f = 1.0;
      for (int i = 0; i < children.size(); ++i) {
        f *= 1.0 / children[i].size();
        p += progress[i] * f;
      }
      fprintf(stderr, "Progress: %6.2lf%% Depth: %ld\n", p * 100,
              children.size());
    }
    return;
  }
  children.emplace_back();
  progress.push_back(0);
  vector<int> &child = children.back();
  for (int i = 0; i < dat_size; ++i) {
    if (i != index && dat[i].check == index) {
      child.push_back(i);
    }
  }
  for (; progress.back() < children.back().size(); ++progress.back()) {
    vector<int> &child = children.back();
    int &i = progress.back();
    int chr = child[i] - base;
    if (chr < 0 || chr > 0x10ffff) {
      continue;
    }
    result.push_back(chr);
    find(child[i]);
    result.pop_back();
  }
  progress.pop_back();
  children.pop_back();
}

int main(int argc, const char *argv[]) {
  assert(argc > 1);
  FILE *file = fopen(argv[1], "rb");
  fseek(file, 0, SEEK_END);
  dat_size = ftell(file) / sizeof(Entry);
  rewind(file);
  dat = (Entry *)calloc(sizeof(Entry), dat_size);
  fread(dat, sizeof(Entry), dat_size, file);
  fclose(file);
  find(0);
  free(dat);
  return 0;
}
