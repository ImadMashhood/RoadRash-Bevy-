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
static void anim(const rr::ActorAnimationState&a){std::cout<<","<<int(a.animation)<<","<<int(a.frame)<<","<<int(a.queued)<<","<<int(a.loops)<<","<<a.deadline<<","<<(a.selected?int(a.selected->at(0)):-1)<<","<<(a.selected?int(a.selected->at(1)):-1);}
static void row(const rr::Race&race,const rr::Rider&r,int tick){
 const auto&b=r.bike;std::cout<<"["<<tick<<","<<race.rng_state();
 for(int off:{0x1c,0x28,0x20,0xec,0xf0,0xf4,0x160,0x23c,0x24,0x11c,0xfc,0x114,0x288,0x2f0,0x2f4})std::cout<<","<<b.i32(off);
 std::cout<<","<<r.stamina.max<<","<<r.stamina.current<<","<<(r.crash_active?r.crash_timer:0)<<","<<(r.crash_active?1:0);anim(r.crash_bike_animation);
 if(r.crash_active){const auto&m=r.crash_actor.motion;for(int off:{0x1c,0x28,0x20,0xec,0xf0,0xf4,0x160,0x24,0x11c,0xfc,0x128,0x5c})std::cout<<","<<m.i32(off);anim(r.crash_actor.animation);}else{for(int i=0;i<12;++i)std::cout<<",0";std::cout<<",-1,0,-1,0,0,-1,-1";}
 std::cout<<"]";
}
int main(int argc,char**argv){try{if(argc!=2)return 2;
 rr::Assets assets(argv[1]);auto exe=assets.read("ROADRASH.EXE");auto tables=rr::read_physics_tables(exe);auto specs=rr::read_bike_specs(assets.read("DATA/BIKESPEC.RSC"));auto crash=rr::read_crash_config(exe);rr::ActorAnimationLibrary library(exe,assets.read("DATA/GLOBAL.RSC"));std::vector<Fixture> fixtures;
 std::cout<<"{\"fixtures\":[";bool first=true;
 for(int kind:{0,1,3,4,255})for(int mode=0;mode<2;++mode){Fixture f{kind,mode,mode?500:0,{},{}};rr::RouteSample rs{{0,0,500,500,512,512},0,0,0};rs.path.shoulder_left=rs.path.shoulder_right=250;rs.path.height_left=16;rs.path.height_right=24;rs.path.elevation=200;rs.offset=f.offset;f.route.assign(512,rs);f.terrain.resize(512);
 for(size_t j=0;j<f.route.size();++j){auto&p=f.route[j].path;f.route[j].local=uint32_t(j);auto&t=f.terrain[j];t.type=uint8_t(kind);t.selectors={0,0};t.walls[0]={100,500,300};t.walls[1]={80,400,400};for(int side=0;side<2;++side)for(int band=0;band<3;++band)t.points[size_t(band*2+side)]={(side?1:-1)*(1000+band*400),band==0?16:band==1?64:-32};
 if(mode){p.bend=int8_t((int(j)%12)<6?12:-18);p.rise=int8_t((int(j)%16)<8?6:-9);p.height_left=uint8_t(8+(j%4)*8);p.height_right=uint8_t(16+(j%3)*12);if(j%11==0){t.selectors={255,255};p.marking_flags=8;}}
 }
 if(!first){std::cout<<",";}first=false;std::cout<<"{\"kind\":"<<kind<<",\"mode\":"<<mode<<",\"offset\":"<<f.offset<<",\"boundaries\":[";for(size_t j=0;j<f.route.size();++j){if(j)std::cout<<",";boundary(f.terrain[j],f.route[j].path,f.offset);}std::cout<<"],\"ground\":[";for(size_t j=0;j<f.route.size();++j){if(j)std::cout<<",";auto&p=f.route[j].path;std::cout<<"["<<-p.left<<","<<p.right<<","<<-p.left-p.shoulder_left<<","<<p.right+p.shoulder_right<<","<<int(f.terrain[j].type)<<","<<int(p.marking_flags)<<","<<int(p.rise)<<","<<int(p.bend)<<"]";}std::cout<<"]}";fixtures.push_back(std::move(f));}
 std::cout<<"],\"cases\":[";first=true;int id=0;const int limit=std::getenv("RR_RECOVERY_LIMIT")?std::atoi(std::getenv("RR_RECOVERY_LIMIT")):100000;
 for(size_t fi=0;fi<fixtures.size();++fi){const auto&f=fixtures[fi];for(int side:{-1,1})for(int zone:std::getenv("RR_RECOVERY_OUTER_ONLY")?std::vector<int>{2}:std::vector<int>{0,1})for(int scenario=0;scenario<8;++scenario){if(id>=limit || (zone==0 && side>0))continue;
 const int reason=scenario==0 || scenario==6?4:scenario==1?2:scenario==3?5:scenario==4?6:scenario==5?3:1;
 const int speed=scenario==0 || scenario==6?0:scenario==1 || scenario==3?8000:scenario==7?-4000:12000;
 const int height=scenario==5?8192:0,vy=scenario==5?-8000:0,condition=scenario==6?0:150;
 const std::array<int32_t,3>rv=reason==4 || reason==5?std::array<int32_t,3>{-80,0,0}:std::array<int32_t,3>{-256,0,1792};const std::array<int32_t,3>bv=reason==4 || reason==5?std::array<int32_t,3>{0,0,0}:std::array<int32_t,3>{0,0,1024};
 rr::Race race(f.route,f.route,tables,specs[0],75);race.set_ride_library(&library,nullptr);race.set_crash_library(library,crash);race.set_live_schedule(true);race.set_terrain_source([&](const rr::RouteSample&p)->const rr::TerrainSlice&{return f.terrain[p.local];});auto&r=race.riders()[0];auto&b=r.bike;r.ai=false;r.frozen=false;r.sample=r.previous_sample=50;r.stamina.base=r.stamina.max=r.stamina.current=2500;r.stamina.strength=7;
 const int position=50*256+128,lateral=f.offset*256+side*(zone==2?1100*256:zone?700*256:0);const int ground=rr::terrain_height(f.terrain[50],f.route[50].path,f.terrain[51],f.route[51].path,lateral-f.offset*256,128,0,f.offset);
 b.i32(0x18)=1;b.i32(0x1c)=b.i32(0x30)=position;b.i32(0x28)=b.i32(0x3c)=lateral;b.i32(0x20)=b.i32(0x34)=height;b.i32(0x24)=b.i32(0x38)=ground;b.i32(0xec)=b.i32(0x108)=speed;b.i32(0xf0)=b.i32(0x10c)=0;b.i32(0xf4)=b.i32(0x110)=vy;b.i32(0x160)=height?0:tables.bike_object[11];b.i32(0x2f0)=condition;b.u8(0xc0)=7;race.set_rng_state(1);race.set_ticks(0);race.crash_reason(r,reason,rv,bv);
 if(!first){std::cout<<",";}first=false;std::cout<<"{\"id\":"<<id++<<",\"fixture\":"<<fi<<",\"scenario\":"<<scenario<<",\"initial\":["<<position<<","<<lateral<<","<<height<<","<<speed<<","<<ground<<","<<condition<<","<<vy<<","<<reason;for(int v:rv)std::cout<<","<<v;for(int v:bv)std::cout<<","<<v;std::cout<<"],\"rows\":[";row(race,r,0);int remount=-1;
 for(int tick=1;tick<=2500;++tick){race.set_ticks(tick-1);r.bike.i32(0x18)=1;r.sched_last=r.half_sched_last=tick-2;race.run_half_node(r,0,false);race.run_bike_node(r,0,false);if(std::getenv("RR_RECOVERY_DIAGNOSTIC") && id==38 && tick<5){auto&m=r.crash_actor.motion;std::cerr<<"beforewalk "<<tick<<" "<<m.i32(0x1c)<<" "<<m.i32(0x128)<<" "<<m.i32(0x184)<<" "<<int(m.u8(0x19c))<<"\n";}race.sort_schedule();race.collide_riders();if(std::getenv("RR_RECOVERY_DIAGNOSTIC") && id==38 && tick<5){auto&m=r.crash_actor.motion;std::cerr<<"afterwalk "<<tick<<" "<<m.i32(0x1c)<<" "<<m.i32(0x128)<<" "<<m.i32(0x184)<<" "<<int(m.u8(0x19c))<<"\n";}std::cout<<",";row(race,r,tick);if(!r.crash_active && remount<0)remount=tick;if(remount>=0 && tick>=remount+30)break;if(scenario==6 && r.crash_actor.animation.animation==37 && r.crash_actor.animation.loops>=1)break;}
 std::cout<<"],\"remount\":"<<remount<<"}";
 }}std::cout<<"]}\n";
 }catch(const std::exception&e){std::cerr<<e.what()<<"\n";return 1;}}
