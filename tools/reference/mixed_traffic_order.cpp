// Actual native mover snapshots followed by real static/rider and traffic scans.
#include "race.hpp"
namespace rr {using AttackCodes=std::array<std::array<std::array<int8_t,4>,2>,2>;AttackCodes read_attack_codes(const std::vector<uint8_t>&);}
#include "assets.hpp"
#include "animation.hpp"
#include "crash.hpp"
#include "traffic.hpp"
#include "track_ring.hpp"
#include <iostream>
void rider_json(const rr::Rider&r){
 const auto&b=r.bike;std::cout<<"[";bool first=true;
 for(int off:{0x1c,0x28,0x20,0x30,0x3c,0x34,0xec,0xf0,0xf4,0x108,0x10c,0x110,0x114,0x288,0x2f0,0x2f4}){if(!first){std::cout<<",";}first=false;std::cout<<b.i32(off);}
 std::cout<<","<<r.stamina.current<<","<<r.stamina.max<<",";
 if(r.crash_active){const auto&m=r.crash_actor.motion;std::cout<<"["<<m.i32(0x1c)<<","<<m.i32(0x28)<<","<<m.i32(0x20)<<","<<m.i32(0xec)<<","<<m.i32(0xf4)<<"]";}else{std::cout<<"null";}std::cout<<"]";
}
void state_json(rr::Race&race,const rr::TrafficSystem&traffic,int cars){
 std::cout<<"[";for(size_t i=0;i<race.riders().size();++i){if(i){std::cout<<",";}rider_json(race.riders()[i]);}
 for(int i=0;i<cars;++i){const auto&car=traffic.cars()[size_t(i)];std::cout<<",["<<car.position<<","<<car.lateral<<","<<car.motion.i32(0x20)<<","<<car.speed<<"]";}std::cout<<"]";
}
void traffic_pass(rr::Race&race,rr::TrafficSystem&traffic,int cars,bool car_outer){
 if(!car_outer){for(auto&r:race.riders()){race.collide_traffic(r);race.collide_traffic_half(r);}return;}
 std::vector<bool>active;for(int i=0;i<cars;++i){active.push_back(traffic.cars()[size_t(i)].active);traffic.cars_mut()[size_t(i)].active=false;}
 for(int i=0;i<cars;++i){traffic.cars_mut()[size_t(i)].active=active[size_t(i)];for(auto&r:race.riders()){race.collide_traffic(r);race.collide_traffic_half(r);}traffic.cars_mut()[size_t(i)].active=false;}
 for(int i=0;i<cars;++i){traffic.cars_mut()[size_t(i)].active=active[size_t(i)];}
}
int main(int argc,char**argv){try{
 if(argc!=2){return 2;}rr::Assets assets(argv[1]);auto exe=assets.read("ROADRASH.EXE");auto tables=rr::read_physics_tables(exe);auto specs=rr::read_bike_specs(assets.read("DATA/BIKESPEC.RSC"));auto crash=rr::read_crash_config(exe);rr::ActorAnimationLibrary library(exe,assets.read("DATA/GLOBAL.RSC"));auto codes=rr::read_attack_codes(exe);rr::VehicleLibrary vehicles(assets.read("DATA/CARS/CITY.CAR"),exe);rr::RouteSample sample{{0,0,2000,2000,512,512},0,0,0};std::vector<rr::RouteSample>route(2000,sample);bool first=true;int id=0;std::cout<<"[";
 for(int elevated:{0,1})for(int kind:{0,1,2,3,16})for(int speed:{0,5000,20000})for(int side:{-1,1})for(int mask:{1,2,4,7})for(int density:{1,2,3}){
  if(elevated&&(speed!=20000||(mask!=2&&mask!=7)||density==1)){continue;}
  const int riders=density==3?3:2,cars=density;
  std::vector<rr::Opponent>opponents;for(int i=1;i<riders;++i){opponents.push_back(rr::Opponent{specs[0],75,0,0,0,i});}
  rr::Race race(route,route,tables,specs[0],75,opponents,0);race.set_ride_library(&library,&codes);race.set_crash_library(library,crash);race.set_live_schedule(true);race.set_ticks(0);
  for(int i=0;i<riders;++i){auto&r=race.riders()[size_t(i)];auto&b=r.bike;r.ai=false;r.frozen=false;r.sched_last=r.half_sched_last=0;r.sample=r.previous_sample=10;r.stamina.base=r.stamina.max=r.stamina.current=2500;r.stamina.strength=7;b.i32(0x18)=1;b.i32(0x1c)=b.i32(0x30)=2688+i*20;b.i32(0x28)=b.i32(0x3c)=i==0?0:i==1?side*10000:-side*12000;b.i32(0x20)=b.i32(0x34)=0;b.i32(0xec)=b.i32(0x108)=speed;b.i32(0x2f0)=150;b.u8(0xc0)=7;
   if(!(mask&4)&&i>0){b.i32(0x1c)=b.i32(0x30)=128000+i*1000;r.sample=r.previous_sample=size_t(b.i32(0x1c)>>8);}
  }
  rr::TrafficSystem traffic(vehicles,exe);
  for(int j=0;j<cars;++j){auto&car=traffic.cars_mut()[size_t(j)];car.active=bool(mask&2);car.collides=true;car.kind=0;car.serial=uint32_t(j+1);car.position=(speed>=20000?2820:2700)+j*20;car.lateral=speed>=20000?(j==0?0:(j%2?side:-side)*4000):(j%2?side:-side)*(10000+j*1000);car.speed=0;car.last_tick=0;car.motion.i32(0xc4)=16;car.motion.u8(0xc0)=7;car.motion.i32(0x1c)=car.motion.i32(0x30)=car.position;car.motion.i32(0x28)=car.motion.i32(0x3c)=car.lateral;car.motion.i32(0xec)=car.motion.i32(0x108)=0;car.motion.i32(0x20)=car.motion.i32(0x34)=elevated&&j>0?4096:0;}
  race.set_traffic(&traffic);rr::AnimationFrame frame;frame.boxes.push_back({-1024,0,1024,4096});race.set_static_frames([&](uint32_t,uint32_t,uint8_t,bool){return &frame;});rr::RingStatics st;if(mask&1){if(kind==16){st.borr_count=1;}else{st.billboards=1;st.billboard[0].type=uint8_t(kind);st.billboard[0].selector=7;}}race.ring_statics(10,0,0,st);race.riders()[0].half_mirror.position=2944;race.prepare_static_boxes();race.apply_static_boxes();
  race.set_rng_state(1);
  for(int i=0;i<riders;++i){race.step_rider(race.riders()[size_t(i)],0,i==0);}race.update_halves();
  if(!first){std::cout<<",";}first=false;std::cout<<"{\"input\":["<<id++<<","<<kind<<","<<speed<<","<<side<<","<<mask<<","<<riders<<","<<cars;if(elevated){std::cout<<",1";}std::cout<<"],\"pre\":";state_json(race,traffic,cars);if(elevated){std::cout<<",\"rng_pre\":"<<race.rng_state();}
  auto car_outer=race;auto car_outer_traffic=traffic;car_outer.set_traffic(&car_outer_traffic);car_outer.collide_riders();traffic_pass(car_outer,car_outer_traffic,cars,true);
  auto legacy=race;auto legacy_traffic=traffic;legacy.set_traffic(&legacy_traffic);traffic_pass(legacy,legacy_traffic,cars,true);legacy.collide_riders();
  race.collide_riders();traffic_pass(race,traffic,cars,false);std::cout<<",\"output\":";state_json(race,traffic,cars);std::cout<<",\"car_outer\":";state_json(car_outer,car_outer_traffic,cars);std::cout<<",\"legacy\":";state_json(legacy,legacy_traffic,cars);if(elevated){std::cout<<",\"rng_output\":"<<race.rng_state()<<",\"rng_car_outer\":"<<car_outer.rng_state()<<",\"rng_legacy\":"<<legacy.rng_state();}std::cout<<"}";
 }
 std::cout<<"]\n";
}catch(const std::exception&e){std::cerr<<e.what()<<"\n";return 1;}}
