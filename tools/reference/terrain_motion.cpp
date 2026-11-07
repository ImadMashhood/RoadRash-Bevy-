// Complete mounted transactions through actual pinned Race::step_rider and the contact list.
#include "race.hpp"
#include "assets.hpp"
#include "animation.hpp"
#include "crash.hpp"
#include "terrain_ground.hpp"
#include <iostream>
struct Fixture {int kind,mode,offset;std::vector<rr::RouteSample> route;std::vector<rr::TerrainSlice> terrain;};
static void boundary(const rr::TerrainSlice&t,const rr::PathSample&p,int offset){
 auto w=rr::wall_profile(t,p,t.type==3);std::cout<<"["<<int(t.type)<<","<<offset<<","<<offset-p.left<<","<<offset+p.right<<","<<offset+w.outer_left<<","<<offset+w.outer_right;
 for(int x:w.base){std::cout<<","<<offset+x;}for(int x:w.top){std::cout<<","<<offset+x;}for(int x:w.height){std::cout<<","<<x;}
 std::cout<<","<<int(t.selectors[0])<<","<<int(t.selectors[1])<<","<<int(p.marking_flags)<<","<<p.height_left<<","<<p.height_right;
 for(int side=0;side<2;++side)for(int band=0;band<3;++band)for(int x:t.points[size_t(band*2+side)])std::cout<<","<<x;
 std::cout<<","<<int(p.event_type)<<","<<int(int8_t(p.event_parameters[1]))<<","<<int(p.event_parameters[2])<<",0]";
}
int main(int argc,char**argv){try{if(argc!=2){return 2;}rr::Assets assets(argv[1]);auto exe=assets.read("ROADRASH.EXE");auto tables=rr::read_physics_tables(exe);auto specs=rr::read_bike_specs(assets.read("DATA/BIKESPEC.RSC"));auto crash=rr::read_crash_config(exe);rr::ActorAnimationLibrary library(exe,assets.read("DATA/GLOBAL.RSC"));std::vector<Fixture> fixtures;
 std::cout<<"{\"fixtures\":[";bool first=true;
 for(int kind:{0,1,3,4,255})for(int mode=0;mode<6;++mode)for(int offset:{0,500}){
  Fixture f{kind,mode,offset,{},{}};rr::RouteSample rs{{0,0,500,500,512,512},0,0,0};rs.offset=offset;rs.path.shoulder_left=rs.path.shoulder_right=250;rs.path.height_left=16;rs.path.height_right=24;rs.path.elevation=200;f.route.assign(128,rs);f.terrain.resize(128);
  for(size_t j=0;j<128;++j){auto&p=f.route[j].path;f.route[j].local=uint32_t(j);auto&t=f.terrain[j];t.type=uint8_t(kind);t.selectors={0,0};t.walls[0]={100,500,300};t.walls[1]={80,400,mode==1?-1:400};
   for(int side=0;side<2;++side)for(int band=0;band<3;++band)t.points[size_t(band*2+side)]={(side?1:-1)*(1000+band*400),band==0?16:band==1?64:-32};
   if(mode==1){if(j==52 || j==53)p.marking_flags=8;if(j%3==0)t.selectors={255,255};}
   if(mode==2){if(j==49){p.marking_flags=8;p.rise=30;}if(j==50)p.rise=-30;if(j==51)p.rise=15;if(j==52)p.rise=-15;}
   if(mode==3 || mode==4){p.event_type=5;p.event_parameters={0,uint8_t(mode==3?12:0),uint8_t((mode==3?28:0)|3)};}
  }
  if(!first){std::cout<<",";}first=false;std::cout<<"{\"kind\":"<<kind<<",\"mode\":"<<mode<<",\"offset\":"<<offset<<",\"boundaries\":[";for(size_t j=0;j<128;++j){if(j)std::cout<<",";boundary(f.terrain[j],f.route[j].path,offset);}std::cout<<"],\"ground\":[";for(size_t j=0;j<128;++j){if(j)std::cout<<",";auto&p=f.route[j].path;std::cout<<"["<<-p.left<<","<<p.right<<","<<-p.left-p.shoulder_left<<","<<p.right+p.shoulder_right<<","<<int(f.terrain[j].type)<<","<<int(p.marking_flags)<<","<<p.rise<<"]";}std::cout<<"]}";fixtures.push_back(std::move(f));
 }
 std::cout<<"],\"cases\":[";first=true;int id=0;
 for(size_t fixture=0;fixture<fixtures.size();++fixture){const auto&f=fixtures[fixture];for(int side:{-1,1})for(int depth:{-1,16,120})for(int speed:{5000,12000,30000})for(int height:{0,8192}){
  rr::Race race(f.route,f.route,tables,specs[0],75);race.set_ride_library(&library,nullptr);race.set_crash_library(library,crash);race.set_terrain_source([&](const rr::RouteSample&p)->const rr::TerrainSlice&{return f.terrain[p.local];});auto&r=race.riders()[0];auto&b=r.bike;r.ai=false;r.frozen=false;r.sample=r.previous_sample=f.mode==2?49:50;r.stamina.base=r.stamina.max=r.stamina.current=2500;r.stamina.strength=7;
  const int position=f.mode==2?50*256-8:50*256+128;const int lateral=f.offset*256+side*(750*256-crash.normal_dimensions[0]+depth*256);const size_t sample=size_t(position>>8);const size_t next=std::min(sample+1,f.route.size()-1);const int ground=rr::terrain_height(f.terrain[sample],f.route[sample].path,f.terrain[next],f.route[next].path,lateral-f.offset*256,uint8_t(position),0,f.offset);
  b.i32(0x18)=1;b.i32(0x1c)=b.i32(0x30)=position;b.i32(0x28)=b.i32(0x3c)=lateral;b.i32(0x20)=b.i32(0x34)=height;b.i32(0x24)=b.i32(0x38)=ground;b.i32(0xec)=b.i32(0x108)=speed;b.i32(0xf0)=b.i32(0x10c)=0;b.i32(0xf4)=b.i32(0x110)=0;b.i32(0x160)=height?0:tables.bike_object[11];b.i32(0x2f0)=150;b.u8(0xc0)=7;
  if(!first){std::cout<<",";}first=false;std::cout<<"{\"id\":"<<id++<<",\"fixture\":"<<fixture<<",\"initial\":["<<position<<","<<lateral<<","<<height<<","<<speed<<","<<ground<<"],\"rows\":[";
  for(int tick=1;tick<=40;++tick){const int steer=side*((f.mode==5 && tick>10 && tick<=20)?-1:1);race.set_ticks(100+tick);const uint32_t controls=steer<0?rr::control_left:rr::control_right;
   auto diagnostic=race;auto&dr=diagnostic.riders()[0];auto&db=dr.bike;diagnostic.fight(dr,controls,0);diagnostic.nitro_charge_tick(dr);diagnostic.rider_think(dr,controls,true);const int surface=rr::surface_class(diagnostic.edges(dr,dr.sample),db.i32(0x28));rr::traction(db,tables,surface,true);if(db.i32(0x2f4)!=0 && !(db.u8(0xc0)&0x10))diagnostic.crash(dr);rr::integrate_speed(db,tables,surface);rr::shift_gears(db);diagnostic.move(dr,true);const std::array<int32_t,5> before_wall={db.i32(0x1c),db.i32(0x28),db.i32(0xec),db.i32(0xf0),db.i32(0x2f4)};diagnostic.walls(dr);const std::array<int32_t,5> after_wall={db.i32(0x1c),db.i32(0x28),db.i32(0xec),db.i32(0xf0),db.i32(0x2f4)};const bool response=before_wall!=after_wall;diagnostic.vertical(dr);diagnostic.collide_riders();
   race.step_rider(r,controls,true);race.collide_riders();for(int off:{0x1c,0x28,0x20,0xec,0xf0,0xf4,0x114,0x288,0x2f0,0x24,0x160,0x11c,0xfc})if(db.i32(off)!=b.i32(off))throw std::runtime_error("Diagnostic native stage walk diverges from actual Race::step_rider");if(tick>1)std::cout<<",";std::cout<<"["<<tick<<","<<steer;for(int off:{0x1c,0x28,0x20,0xec,0xf0,0xf4,0x114,0x288,0x2f0,0x24,0x160})std::cout<<","<<b.i32(off);std::cout<<","<<int(b.u8(0x164));for(int off:{0x11c,0xfc})std::cout<<","<<b.i32(off);std::cout<<","<<r.stamina.max<<","<<r.stamina.current<<","<<(r.crash_active?1:0);if(r.crash_active){auto&m=r.crash_actor.motion;for(int off:{0x1c,0x28,0x20,0xec,0xf0,0xf4})std::cout<<","<<m.i32(off);std::cout<<","<<int(r.crash_actor.animation.animation);}else{std::cout<<",0,0,0,0,0,0,0";}std::cout<<","<<(response?1:0)<<"]";if(r.crash_active)break;}
  std::cout<<"]}";
 }}std::cout<<"]}\n";
}catch(const std::exception&e){std::cerr<<e.what()<<"\n";return 1;}}
