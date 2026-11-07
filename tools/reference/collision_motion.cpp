// Fixed-dt mounted trajectories: ordinary motion and explicitly injected validated speed extremes.
#include "race.hpp"
#include "assets.hpp"
#include "crash.hpp"
#include "animation.hpp"
#include "track_ring.hpp"
#include <iostream>
#include <map>
#include <cmath>
#include <tuple>
int main(int argc,char**argv){try{
 if(argc!=2){return 2;}rr::Assets assets(argv[1]);auto tables=rr::read_physics_tables(assets.read("ROADRASH.EXE"));auto specs=rr::read_bike_specs(assets.read("DATA/BIKESPEC.RSC"));auto crash=rr::read_crash_config(assets.read("ROADRASH.EXE"));rr::ActorAnimationLibrary library(assets.read("ROADRASH.EXE"),assets.read("DATA/GLOBAL.RSC"));
 bool first=true;int id=0;std::map<std::tuple<size_t,int,int>,int> anchors;std::cout<<"[";
 for(size_t spec=0;spec<specs.size();++spec)for(int steering:{-1,0,1})for(int terrain=0;terrain<3;++terrain)for(int obstacle=0;obstacle<4;++obstacle)for(int initial_speed:{0,30000,1000000}){
  if(initial_speed==0 && terrain!=0)continue;
  if(obstacle==3 && (terrain!=0 || steering==0 || initial_speed==1000000))continue;
  const int anchor=obstacle==3?anchors.at({spec,steering,initial_speed}):0;
  rr::RouteSample sample{{0,0,5000,5000,512,512},0,0,0};std::vector<rr::RouteSample> route(2000,sample);for(size_t j=0;j<route.size();++j){route[j].local=uint32_t(j);route[j].path.shoulder_left=route[j].path.shoulder_right=250;if(terrain && j>=54 && j<58)route[j].path.rise=50;if(terrain==2 && j>=58 && j<62)route[j].path.rise=-50;}
  rr::Race race(route,route,tables,specs[spec],75);race.set_ride_library(&library,nullptr);race.set_crash_library(library,crash);auto&r=race.riders()[0];auto&b=r.bike;r.ai=false;r.frozen=false;r.sample=r.previous_sample=50;r.stamina.base=r.stamina.max=r.stamina.current=2500;r.stamina.strength=7;
  b.i32(0x18)=1;b.i32(0x1c)=b.i32(0x30)=12800;b.i32(0xec)=b.i32(0x108)=initial_speed;b.i32(0x28)=b.i32(0x3c)=0;b.i32(0x20)=b.i32(0x34)=0;b.i32(0x2f0)=150;b.u8(0xc0)=7;
  if(initial_speed==0){for(int tick=1;tick<=3000;++tick){race.set_ticks(100+tick);race.step_rider(r,rr::control_accelerate,true);}r.sample=r.previous_sample=50;b.i32(0x1c)=b.i32(0x30)=12800;}
  rr::AnimationFrame frame;frame.boxes.push_back({-1024,obstacle==2?12288:0,1024,obstacle==2?16384:4096});race.set_static_frames([&](uint32_t,uint32_t,uint8_t,bool){return &frame;});if(obstacle){rr::RingStatics st;st.billboards=1;st.billboard[0].type=0;st.billboard[0].offset=anchor;race.ring_statics(60,0,60,st);}
  for(int tick=1;tick<=90;++tick){race.set_ticks(100+tick);uint32_t controls=rr::control_accelerate;if(tick<=20)controls|=steering<0?rr::control_left:steering>0?rr::control_right:0;const int old_x=b.i32(0x1c),old_y=b.i32(0x28);race.step_rider(r,controls,true);race.collide_riders();
   if(obstacle==0 && terrain==0 && initial_speed!=1000000 && steering!=0 && old_x<15424 && b.i32(0x1c)>=15424){const int dy=b.i32(0x28)-old_y;const double enter=double(15424-old_x)/double(b.i32(0x1c)-old_x);const double raw=(old_y+dy*enter/2-(dy>0?1:-1)*(crash.normal_dimensions[0]+1024))/256.;anchors[{spec,steering,initial_speed}]=int(dy>0?std::floor(raw):std::ceil(raw));}
   if(!first){std::cout<<",";}first=false;std::cout<<"["<<id<<","<<spec<<","<<steering<<","<<terrain<<","<<obstacle<<","<<tick;for(int off:{0x1c,0x28,0x20,0xec,0xf0,0xf4,0x114,0x288,0x2f0})std::cout<<","<<b.i32(off);std::cout<<","<<(r.crash_active?1:0)<<","<<initial_speed<<","<<anchor<<"]";if(r.crash_active)break;}++id;
 }std::cout<<"]\n";
}catch(const std::exception&e){std::cerr<<e.what()<<"\n";return 1;}}
