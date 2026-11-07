// Controlled live-list fight transactions; no original pixels/data embedded.
#include "race.hpp"
#include "assets.hpp"
#include "crash.hpp"
#include "actor_animation.hpp"
#include <iostream>
namespace rr {using AttackCodes=std::array<std::array<std::array<int8_t,4>,2>,2>;AttackCodes read_attack_codes(const std::vector<uint8_t>&);}
int main(int argc,char**argv){try{
 if(argc<2){return 2;}
 const bool quick=argc>2;
 rr::Assets assets(argv[1]);const auto exe=assets.read("ROADRASH.EXE");auto tables=rr::read_physics_tables(exe);auto specs=rr::read_bike_specs(assets.read("DATA/BIKESPEC.RSC"));
 rr::ActorAnimationLibrary library(exe,assets.read("DATA/GLOBAL.RSC"));const auto codes=rr::read_attack_codes(exe);const auto crash=rr::read_crash_config(exe);
 rr::RouteSample sample{{0,0,500,500,512,512},0,0,0};std::vector<rr::RouteSample>route(2000,sample);
 std::cout<<"[";bool first_case=true;int case_id=0;
 for(int count:{3,4,8})for(int weapon=0;weapon<4;++weapon)for(int kind=0;kind<2;++kind)for(int side:{-1,1})for(int mode=0;mode<8;++mode)for(uint32_t seed:{3u,4u}){
  if(weapon==3&&kind){continue;}
  if(quick&&(count!=3||weapon!=0||kind!=0||side!=1||mode>2))continue;
  std::vector<rr::Opponent>opponents(size_t(count-1),rr::Opponent{specs[0],75,0,0,0,1});rr::Race race(route,route,tables,specs[0],75,opponents,0,0,{},exe);
  race.set_ride_library(&library,&codes);race.set_crash_library(library,crash);race.live_schedule_=true;
  for(int a=0;a<count;++a){auto&r=race.riders()[size_t(a)];r.ai=false;r.frozen=false;r.cop=mode==5&&a==count-1;r.stamina.base=r.stamina.max=r.stamina.current=2500;r.stamina.strength=1;if(mode==6&&a==0)r.stamina.current=64;r.weapon=weapon==3?0:weapon;r.attack.weapon=r.weapon;
   r.bike.i32(0x18)=1;r.bike.i32(0x2f0)=150;r.bike.i32(0x29c)=11776;r.bike.i32(0x2d8)=0;
   const auto&bank=library.playback[r.cop?1:0];rr::set_animation_with_fallback(r.ride_animation,bank,2,0,library.fallback);rr::queue_animation_with_fallback(r.ride_animation,bank,2,library.fallback);
   if(mode==4){r.weapon=a==1?1:0;r.attack.weapon=r.weapon;}
  }
  race.set_rng_state(seed);
  if(!first_case){std::cout<<",";}
  first_case=false;
  std::cout<<"{\"id\":"<<case_id++<<",\"count\":"<<count<<",\"weapon\":"<<weapon<<",\"kind\":"<<kind<<",\"side\":"<<side<<",\"mode\":"<<mode<<",\"seed\":"<<seed<<",\"rows\":[";
  for(int tick=0;tick<96;++tick){race.set_ticks(tick);race.previous_camera_position_=744;
   for(int a=0;a<count;++a){auto&r=race.riders()[size_t(a)];int gap=a==0?0:(a%2?16:-16);if(mode==1||(mode==7&&tick>=24))gap=0;if(mode==2&&a==1)gap=tick<12?64:tick<48?63:64;if(mode==3&&a==count-1)gap=tick<24?-8:1000;
    const int lat=side*((a%3)-1)*16000;const int height=mode==2&&a==2?(tick<24?12288:12287):0;
    r.sample=r.previous_sample=size_t((1000+gap)>>8);r.bike.i32(0x1c)=1000+gap;r.bike.i32(0x28)=lat;r.bike.i32(0x20)=height;r.bike.i32(0xec)=100;r.bike.i32(0x288)=0;
    r.half_mirror.position=r.crash_active?r.crash_actor.motion.i32(0x1c):1000+gap;r.half_mirror.lateral=r.crash_active?r.crash_actor.motion.i32(0x28):lat;r.half_mirror.height=r.crash_active?r.crash_actor.motion.i32(0x20):height;r.half_mirror.sample=r.sample;
   }
   race.sort_schedule();
   for(int a=0;a<count;++a){auto&r=race.riders()[size_t(a)];const bool held=mode==4?(a==1?tick>=10:tick<12||tick>=40):(tick==0||mode==0||mode==1||tick%40==0);
    const uint32_t command=held?(weapon==3?rr::control_kick:kind?rr::control_backhand:rr::control_punch):0;
    race.fight(r,command,size_t(a));race.advance_ride_animation(r,a==0);
   }
   if(tick){std::cout<<",";}
   std::cout<<"["<<tick<<","<<race.rng_state();
   for(const auto&r:race.riders())std::cout<<","<<r.stamina.max<<","<<r.stamina.current<<","<<r.bike.i32(0x2f0)<<","<<r.bike.i32(0xec)<<","<<r.bike.i32(0x288)<<","<<r.hit_until<<","<<r.steal_until<<","<<r.weapon<<","<<int(r.ride_animation.animation)<<","<<int(r.ride_animation.frame)<<","<<int(r.ride_animation.queued)<<","<<int(r.ride_animation.loops)<<","<<(r.crash_active?1:0)<<","<<(r.crash_active?int(r.crash_actor.animation.animation):-99)<<","<<(r.crash_active&&r.crash_actor.animation.selected?1:0)<<","<<(r.crash_bike_animation.selected?1:0);
   std::cout<<"]";
  }
  std::cout<<"]}";
 }
 std::cout<<"]\n";return 0;
}catch(const std::exception&e){std::cerr<<e.what()<<"\n";return 1;}}
