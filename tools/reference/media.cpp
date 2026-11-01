// Independently authored public API adapter around pinned native readers and sound scalars.
#include "assets.hpp"
#include "wav.hpp"
#include "avi.hpp"
#include "ima_adpcm.hpp"
#include "bike.hpp"
#include <cstdint>
namespace rr { int pitch_q8(int32_t frequency,int source_rate); }
#include <filesystem>
#include <fstream>
#include <iostream>
#include <sstream>
#include <iomanip>
#include <algorithm>
uint64_t hash_samples(const std::vector<int16_t>& v){uint64_t h=0xcbf29ce484222325ULL;for(auto s:v){uint16_t u=uint16_t(s);for(int shift:{0,8})h=(h^uint8_t(u>>shift))*0x100000001b3ULL;}return h;}
std::string hex_hash(const std::vector<int16_t>& v){std::ostringstream s;s<<std::hex<<std::setw(16)<<std::setfill('0')<<hash_samples(v);return s.str();}
int main(int argc,char**argv){try{
 if(argc!=3){return 2;}rr::Assets assets(argv[1]);std::filesystem::path root(argv[1]),out(argv[2]);std::vector<std::string> paths;
 for(const char* dir:{"AUDIO/EFFECTS","AUDIO/MUSIC"}){for(const auto&e:std::filesystem::directory_iterator(root/dir)){auto ext=e.path().extension().string();if(ext==".WAV"||ext==".RRA")paths.push_back(std::filesystem::relative(e.path(),root).generic_string());}}
 std::sort(paths.begin(),paths.end());
 std::cout<<"{\"wav\":[";bool first=true;for(const auto&p:paths){auto s=rr::read_wav(assets.read(p));if(!first){std::cout<<",";}first=false;std::cout<<"[\""<<p<<"\","<<s.rate<<","<<s.channels<<","<<s.samples.size()<<",\""<<hex_hash(s.samples)<<"\"]";}
 std::cout<<"],\"rpm\":[";first=true;for(int current:{0,8192,16000,40000})for(int prev:{0,10000})for(int ratio:{256,1024,4096})for(int speed:{0,1,10000,30000})for(int height:{0,101})for(bool crashed:{false,true}){
 rr::Bike b;b.i32(0x2dc)=current;b.i32(0x2e8)=prev;b.i32(0x1cc)=ratio;b.i32(0x23c)=0;b.i32(0xec)=speed;b.i32(0x20)=height;b.i32(0x2f4)=crashed?1:0;rr::update_engine_rpm(b);if(!first){std::cout<<",";}first=false;std::cout<<"["<<int32_t(b.i32(0x2dc))<<","<<int32_t(b.i32(0x2e8))<<"]";}
 std::cout<<"],\"pitch\":[";first=true;for(int frequency:{-1,0,100,8192,16000,100000,200000})for(int rate:{8000,22050,44100,48000}){if(!first){std::cout<<",";}first=false;std::cout<<rr::pitch_q8(frequency,rate);}
 std::cout<<"],\"avi\":[";first=true;for(const char* name:{"WIN1","BUSTED1","WRECK1","INTRO"}){
 auto bytes=assets.read(std::string("VIDEO/")+name+".AVI");auto avi=rr::read_avi(bytes);size_t video=0,audio=0;for(size_t i=0;i<avi.streams.size();++i){if(avi.streams[i].type==rr::fourcc('v','i','d','s'))video=i;if(avi.streams[i].type==rr::fourcc('a','u','d','s'))audio=i;}auto v=avi.streams[video],a=avi.streams[audio];size_t frames=0;std::vector<int16_t> pcm;
 for(const auto&c:avi.chunks){if(c.stream==int(video))++frames;if(c.stream==int(audio)){auto block=rr::decode_ima_adpcm(bytes.data()+c.offset,c.size,a.audio_channels,a.block_align);pcm.insert(pcm.end(),block.begin(),block.end());}}
 std::ofstream raw(out/(std::string("native-")+name+".pcm"),std::ios::binary);for(auto s:pcm){uint16_t u=uint16_t(s);raw.put(char(u&255));raw.put(char(u>>8));}
 if(!first){std::cout<<",";}first=false;std::cout<<"[\""<<name<<"\","<<v.width<<","<<v.height<<","<<v.rate<<","<<v.scale<<","<<frames<<","<<pcm.size()<<",\""<<hex_hash(pcm)<<"\"]";
 }
 std::cout<<"]}\n";
}catch(std::exception&e){std::cerr<<e.what()<<"\n";return 1;}}
