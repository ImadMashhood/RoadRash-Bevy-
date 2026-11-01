// Compare every indexed runtime rider identity through the public source API.
#include "rider_sprites.hpp"
#include "sha256.hpp"
#include <iostream>
int main(int argc,char**argv){try{if(argc!=2)return 2;rr::Assets a(argv[1]);auto sprites=rr::load_rider_sprites(a,a.read("ROADRASH.EXE"),true);bool first=true;std::cout<<"[";for(int color=0;color<16;++color)for(int frame=0;frame<326;++frame){auto t=rr::rider_texture(sprites,color==15?0xe0:color,frame);if(!t)continue;std::vector<uint8_t> pixels;int w=t->bounds[2],h=t->bounds[3];for(int y=0;y<h;++y)for(int x=0;x<w;++x)pixels.push_back(t->mips[0].pixels[size_t(y)*256+size_t(x)]);if(!first){std::cout<<",";}first=false;std::cout<<"["<<(color==15?0xe0:color)<<","<<frame<<","<<w<<","<<h<<",\""<<rr::sha256(pixels)<<"\"]";}std::cout<<"]\n";}catch(std::exception&e){std::cerr<<e.what()<<"\n";return 1;}}
