// Controlled actual native list-walk transactions. Linked reference stays unmodified.
#include "race.hpp"
#include "assets.hpp"
#include "crash.hpp"
#include "animation.hpp"
#include "track_ring.hpp"
#include <iostream>
int main(int argc,char**argv){try{
 if(argc!=2)return 2;
 rr::Assets assets(argv[1]);auto tables=rr::read_physics_tables(assets.read("ROADRASH.EXE"));
 auto specs=rr::read_bike_specs(assets.read("DATA/BIKESPEC.RSC"));
 auto crash=rr::read_crash_config(assets.read("ROADRASH.EXE"));
 rr::ActorAnimationLibrary library(assets.read("ROADRASH.EXE"),assets.read("DATA/GLOBAL.RSC"));
 rr::RouteSample sample{{0,0,500,500,512,512},0,0,0};std::vector<rr::RouteSample> route(2000,sample);
 bool first=true;int id=0;std::cout<<"[";
 for(int count:{3,4,8,16})for(int layout=0;layout<24;++layout)for(int speed:{0,5000,30000})for(int solid:{0,1}){
  std::vector<rr::Opponent> opponents;for(int i=1;i<count;++i)opponents.push_back({specs[0],75,0,0,0,i});
  rr::Race race(route,route,tables,specs[0],75,opponents,0);race.set_crash_library(library,crash);race.set_ticks(100);race.set_rng_state(3);
  rr::AnimationFrame frame;frame.boxes.push_back({-1024,0,1024,4096});
  race.set_static_frames([&](uint32_t,uint32_t,uint8_t,bool){return &frame;});
  if(solid){rr::RingStatics st;st.billboards=1;st.billboard[0].type=0;race.ring_statics(10,0,0,st);}
  if(!first){std::cout<<",";}first=false;std::cout<<"["<<id++<<","<<count<<","<<layout<<","<<speed<<","<<solid;
  for(int i=0;i<count;++i){auto&r=race.riders()[size_t(i)];auto&b=r.bike;
   int x=2688+(i-1)*(layout<4?20:layout<8?120:0);int y=(layout%4==0?0:(i-1)*(layout%4==1?1500:6000));int z=layout>=20?(i==2?19200:0):layout>=16?0:layout>=8?i*(crash.normal_dimensions[2]*2/(count-1)+layout%4-1):layout%4==3?i*1000:0;
   r.ai=true;r.frozen=false;r.sample=r.previous_sample=size_t(x>>8);r.stamina.base=r.stamina.max=r.stamina.current=2500;r.stamina.strength=7;
   b.i32(0x18)=1;b.i32(0x1c)=x;b.i32(0x28)=y;b.i32(0x20)=z;b.i32(0x30)=layout>=16?(i?x:x-129):x-speed/256;b.i32(0x3c)=y;b.i32(0x34)=z;
   for(int j=0;j<3;++j)b.i32(0x174+j*4)=crash.normal_dimensions[size_t(j)];
   b.i32(0xec)=b.i32(0x108)=i==0?speed:5000;b.i32(0xf0)=b.i32(0x10c)=0;b.i32(0xf4)=b.i32(0x110)=0;
   b.i32(0x2f0)=150;b.i32(0xc4)=4;b.u8(0xc0)=7;
  }
  if(layout>=12 && layout<16){auto&r=race.riders()[0];auto&b=r.bike;r.crash_active=true;b.i32(0x2f4)=1;
   for(int j=0;j<3;++j)b.i32(0x174+j*4)=crash.bike_dimensions[size_t(j)];
   rr::init_crash_actor(r.crash_actor,crash);auto&m=r.crash_actor.motion;
   m.i32(0x1c)=m.i32(0x30)=b.i32(0x1c)+10;m.i32(0x28)=m.i32(0x3c)=b.i32(0x28);m.i32(0x20)=m.i32(0x34)=0;
   m.i32(0xec)=m.i32(0x108)=speed;m.i32(0xf0)=m.i32(0x10c)=0;m.i32(0xf4)=m.i32(0x110)=0;m.u8(0xc0)=7;
   r.crash_actor.sample=r.crash_actor.previous_sample=10;rr::set_animation_with_fallback(r.crash_actor.animation,library.playback[0],33,100,library.fallback);race.sync_half_mirror(r);
  }
  race.collide_riders();
  for(auto&r:race.riders()){auto&b=r.bike;std::cout<<","<<b.i32(0x1c)<<","<<b.i32(0x28)<<","<<b.i32(0x20)<<","<<b.i32(0xec)<<","<<b.i32(0xf0)<<","<<b.i32(0xf4)<<","<<b.i32(0x2f0)<<","<<r.stamina.max<<","<<r.stamina.current<<","<<(r.crash_active?1:0);}
  if(layout>=12){auto&r=race.riders()[0];auto&m=r.crash_actor.motion;for(int off:{0x1c,0x28,0x20,0xec,0xf0,0xf4})std::cout<<","<<(r.crash_active?m.i32(off):0);}
  std::cout<<"]";
 }std::cout<<"]\n";
}catch(const std::exception&e){std::cerr<<e.what()<<"\n";return 1;}}
