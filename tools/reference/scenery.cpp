// Authored inputs drive the actual pinned native collision and loading APIs.
#include "race.hpp"
#include "assets.hpp"
#include "animation.hpp"
#include "track_ring.hpp"
#include "crash.hpp"
#include "actor_descriptors.hpp"
#include <iostream>
int main(int argc,char**argv){try{
 if(argc!=2)return 2;
 rr::Assets assets(argv[1]);const auto exe=assets.read("ROADRASH.EXE");
 auto tables=rr::read_physics_tables(exe);auto specs=rr::read_bike_specs(assets.read("DATA/BIKESPEC.RSC"));
 rr::ActorAnimationLibrary library(exe,assets.read("DATA/GLOBAL.RSC"));auto crash=rr::read_crash_config(exe);
 rr::RouteSample sample{{0,0,2000,2000,512,512},0,0,0};std::vector<rr::RouteSample> route(2000,sample);
 bool first=true;int id=0;std::cout<<"[";
 auto run=[&](int mode,int kind,int slot,int flags,int count,int lateral,int speed,int fallen,int ready,int camera,int repeat){
  auto case_route=route;
  auto other_route=route;
  if(mode==8){for(size_t i=128;i<case_route.size();++i){case_route[i].node=other_route[i].node=1;case_route[i].section=other_route[i].section=1;case_route[i].local=other_route[i].local=uint32_t(i-128);}other_route[10].section=2;}
  if(mode==5)for(size_t i=128;i<case_route.size();++i){case_route[i].node=1;case_route[i].section=1;case_route[i].local=uint32_t(i-128);}
  rr::Race race(case_route,mode==8?other_route:case_route,tables,specs[0],75);race.set_ride_library(&library,nullptr);race.set_crash_library(library,crash);race.set_ticks(100);
  rr::AnimationFrame frame;frame.boxes.push_back({-1024,0,1024,4096});if(count==2)frame.boxes.push_back({4096,0,6144,4096});
  auto texture=std::make_shared<rr::CelTexture>();texture->flags=uint32_t(flags);frame.texture=texture;
  race.set_static_frames([&](uint32_t,uint32_t,uint8_t,bool){return &frame;});
  rr::RingStatics st;if(kind==16){st.borr_count=1;st.borr[0]=slot;}else{st.billboards=1;st.billboard[0].type=uint8_t(kind);st.billboard[0].selector=7;st.billboard[0].offset=slot;}
  if(mode==2){st.borr_count=1;st.borr[0]=slot;st.billboards=1;st.billboard[0].type=3;st.billboard[0].selector=9;st.billboard[0].offset=slot;}
  const int target=(mode==4?40:(mode==5 || mode==8)?138:10),position=target*256+128;
  race.ring_statics(target,(mode==5 || mode==8)?1:0,(mode==5 || mode==8)?10:0,st);
  if(mode==5){race.ring_statics(10,0,0,st);race.riders()[0].half_mirror.position=2688+256;race.prepare_static_boxes();race.apply_static_boxes();frame.texture.reset();race.ring_statics(138,1,10,st);}
  if(mode==8){
   rr::AnimationFrame frame0=frame,frame1=frame;auto t0=std::make_shared<rr::CelTexture>();auto t1=std::make_shared<rr::CelTexture>();t1->flags=uint32_t(flags);frame0.texture=t0;frame1.texture=t1;
   race.set_static_frames([&](uint32_t section,uint32_t,uint8_t,bool){return section==0?&frame0:section==2?&frame1:&frame;});race.ring_statics(10,0,0,st);race.ring_statics(10,2,0,st);race.riders()[0].half_mirror.position=2688+256;race.prepare_static_boxes();race.apply_static_boxes();
   frame.texture.reset();race.ring_statics(138,1,10,st);
   race.set_static_frames([&](uint32_t,uint32_t,uint8_t,bool){return &frame;});
   // set_static_frames resets slices; the native ring-slot words persist.
   race.ring_statics(138,1,10,st);
  }
  if(mode==3 || mode==4){race.set_live_schedule(true);race.riders()[0].sched_last=100;}
  auto&r=race.riders()[0];auto&b=r.bike;r.ai=false;r.frozen=false;r.route=mode==8?1:0;r.sample=r.previous_sample=size_t(target);
  r.stamina.base=r.stamina.max=r.stamina.current=2500;r.stamina.strength=7;
  b.i32(0x18)=1;b.i32(0x1c)=b.i32(0x30)=position;b.i32(0x28)=b.i32(0x3c)=lateral;b.i32(0x20)=b.i32(0x34)=0;
  const auto&dims=fallen?crash.bike_dimensions:crash.normal_dimensions;for(int j=0;j<3;++j)b.i32(0x174+j*4)=dims[size_t(j)];
  b.i32(0xec)=b.i32(0x108)=speed;b.i32(0xf0)=256;b.i32(0x10c)=0;b.i32(0x114)=256;b.i32(0x288)=384;b.i32(0x2f4)=fallen;b.i32(0x2f0)=150;b.i32(0xc4)=4;b.u8(0xc0)=7;b.u8(0x19c)=0x80;
  r.half_mirror.position=camera+256;
  if(ready && mode!=7 && mode!=10){race.prepare_static_boxes();if(ready>1)race.apply_static_boxes();}
  if(mode==6){race.countdown_step();race.countdown_step();r.sched_last=race.ticks();}
  race.collide_riders();
  if(mode==10){auto other=r;race.contact(r,other);}
  if(mode==7 || mode==10){for(int age=1;age<=ready;++age){race.set_ticks(100+age);if(camera==1 && age==3){b.i32(0x1c)+=256;r.sample=size_t(b.i32(0x1c)>>8);}race.step_rider(r,0,true);}}
  if(repeat && mode!=7 && mode!=10){race.set_ticks(101);r.sched_last=101;b.i32(0x1c)=b.i32(0x30)=position;b.i32(0x28)=b.i32(0x3c)=lateral;race.collide_riders();}
  if(!first){std::cout<<",";}first=false;
  std::cout<<"["<<id++<<","<<mode<<","<<kind<<","<<slot<<","<<flags<<","<<count<<","<<lateral<<","<<speed<<","<<fallen<<","<<ready<<","<<camera<<","<<repeat;
  for(int off:{0x1c,0x28,0x20,0xec,0x288,0xf0,0x2f0,0x298})std::cout<<","<<b.i32(off);
  std::cout<<","<<b.i32(0x114)<<","<<(b.u8(0x19c)!=0x80)<<","<<r.half_object.i32(0x1ec)<<","<<int(b.u8(0x80))<<","<<int(b.u8(0x81))<<","<<int(r.half_object.u8(0x1e9))<<"]";
 };
 for(int slot:{-1024,-1023,0,1023,1024})for(int flags:{0,0x4000})for(int count:{1,2})for(int edge:{-1,0,1})run(0,16,slot,flags,count,slot*256+7680+3072+edge,0,0,2,2688,0);
 for(int kind=3;kind<16;++kind)for(int speed:{0,1000,1001,30000})for(int fallen:{0,1})for(int repeat:{0,1})run(1,kind,0,0,1,0,speed,fallen,2,2688,repeat);
 for(int flags:{0,0x4000})for(int lateral:{0,10000})run(2,16,0,flags,1,lateral,5000,0,2,2688,0);
 for(int ready:{0,1,2})for(int camera:{0,2688-0x1e00,2688-0x1e00-1,2688,3071,3072})run(3,16,0,0,1,0,0,0,ready,camera,0);
 for(int ready:{0,1,2})for(int camera:{0,2560,2559,10368,10751,10752})run(4,16,0,0,1,0,0,0,ready,camera,0);
 for(int flags:{0,0x4000})run(5,16,1024,flags,1,1024*256+7680+3072,0,0,2,138*256+128,0);
 run(6,16,0,0,1,0,0,0,0,2688,0);
 for(int speed:{1001,5000})for(int age=0;age<=16;++age)for(int leave:{0,1})run(7,3,0,0,1,0,speed,0,age,leave,0);
 for(int age:{0,1,4,12})run(10,3,0,0,1,0,5000,0,age,0,0);
 for(int flags:{0,0x4000})run(8,16,1024,flags,1,1024*256+7680+3072,0,0,2,138*256+128,0);
 {
  rr::Course course;course.root=20;rr::CourseSection section;section.samples.assign(2000,sample.path);course.sections.push_back(section);course.nodes.emplace(20,rr::CourseNode{20,0,60,52,0,2000});course.nodes.emplace(52,rr::CourseNode{52,3});course.nodes.emplace(60,rr::CourseNode{60,4});
  auto moving_route=route;for(size_t i=0;i<moving_route.size();++i){moving_route[i].node=20;moving_route[i].local=uint32_t(i);}
  rr::Opponent opponent{specs[0],75,0,0,0,1};rr::Race race(moving_route,moving_route,tables,specs[0],75,{opponent},0);race.set_ride_library(&library,nullptr);race.set_crash_library(library,crash);
  for(size_t i=0;i<2;++i){auto&r=race.riders()[i];auto&b=r.bike;r.ai=false;r.frozen=false;r.sample=r.previous_sample=i?81:50;b.i32(0x1c)=b.i32(0x30)=i?20864:12800;b.i32(0xec)=b.i32(0x108)=i?0:30000;b.i32(0x28)=b.i32(0x3c)=0;b.i32(0x20)=b.i32(0x34)=0;b.i32(0x2f0)=150;b.u8(0xc0)=7;}
  race.set_live_schedule(true);for(auto&r:race.riders()){r.sched_last=r.half_sched_last=0;r.bike.i32(0x18)=1;}
  rr::TrackRing ring(course,0);race.set_track_ring(&ring);
  rr::AnimationFrame frame;frame.boxes.push_back({-1024,0,1024,4096});race.set_static_frames([&](uint32_t,uint32_t,uint8_t,bool){return &frame;});rr::RingStatics st;st.borr_count=1;race.ring_statics(81,0,81,st);
  for(int tick=1;tick<=12;++tick){race.step(0);if(ring.unsupported())throw std::runtime_error("Synthetic straight native ring unsupported");if(!first){std::cout<<",";}first=false;std::cout<<"["<<id++<<",9,"<<tick<<","<<race.riders()[0].bike.i32(0x1c)<<","<<ring.front_q8()-0x5000<<","<<int(race.static_slice_state(0,81).boxes_ready)<<"]";}
 }
 {
  rr::Course course;course.root=20;rr::CourseSection section;section.samples.assign(2000,sample.path);course.sections.push_back(section);course.nodes.emplace(20,rr::CourseNode{20,0,60,52,0,2000});course.nodes.emplace(52,rr::CourseNode{52,3});course.nodes.emplace(60,rr::CourseNode{60,4});
  auto moving_route=route;for(size_t i=0;i<moving_route.size();++i){moving_route[i].node=20;moving_route[i].local=uint32_t(i);}
  rr::Race race(moving_route,moving_route,tables,specs[0],75);race.riders()[0].half_mirror.position=50*256;
  rr::AnimationFrame frame;frame.boxes.push_back({-1024,0,1024,4096});race.set_static_frames([&](uint32_t,uint32_t,uint8_t,bool){return &frame;});
  rr::TrackRing ring(course,0);ring.set_static_sink([&](int32_t sample,uint32_t section,uint32_t local,const rr::RingStatics&){rr::RingStatics st;if(local==81){st.borr_count=1;st.borr[0]=1776;}race.ring_statics(sample,section,local,st);});race.set_track_ring(&ring);
  int previous=50;for(int frame_no=0;frame_no<=240;++frame_no){int camera=frame_no<=120?50+frame_no:290-frame_no;race.apply_static_boxes();race.riders()[0].half_mirror.position=camera*256;ring.advance((camera-previous)*256,[]{return 1u;},[](const rr::RingActor&,uint32_t){});previous=camera;race.prepare_static_boxes();if(ring.unsupported())throw std::runtime_error("Reverse ring unsupported");if(!first)std::cout<<",";first=false;std::cout<<"["<<id++<<",11,"<<frame_no<<","<<camera*256<<","<<int(race.static_slice_state(0,81).boxes_ready)<<"]";}
 }
 std::cout<<"]\n";return 0;
}catch(const std::exception&e){std::cerr<<e.what()<<"\n";return 1;}}
