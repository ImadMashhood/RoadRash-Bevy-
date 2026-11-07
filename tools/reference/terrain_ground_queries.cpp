#include "assets.hpp"
#include "bike.hpp"
#include "course_scene.hpp"
#include "terrain_ground.hpp"
#include <iostream>
static void boundary(const rr::TerrainSlice& t,const rr::PathSample& p,int offset,int rail=0){
 auto w=rr::wall_profile(t,p,t.type==3);
 std::cout<<"["<<int(t.type)<<","<<offset<<","<<offset-p.left<<","<<offset+p.right<<","<<offset+w.outer_left<<","<<offset+w.outer_right;
 for(int x:w.base){std::cout<<","<<offset+x;}for(int x:w.top){std::cout<<","<<offset+x;}for(int x:w.height){std::cout<<","<<x;}
 std::cout<<","<<int(t.selectors[0])<<","<<int(t.selectors[1])<<","<<int(p.marking_flags)<<","<<p.height_left<<","<<p.height_right;
 for(int side=0;side<2;++side)for(int band=0;band<3;++band)for(int x:t.points[size_t(band*2+side)])std::cout<<","<<x;
 std::cout<<","<<int(p.event_type)<<","<<int(int8_t(p.event_parameters[1]))<<","<<int(p.event_parameters[2])<<","<<rail<<"]";
}
int main(int argc,char**argv){try{
 if(argc!=2){return 2;}rr::Assets assets(argv[1]);auto exe=assets.read("ROADRASH.EXE");
 std::cout<<"{\"ground\":[";bool first=true;int id=0;
 for(int kind:{0,1,3,4,255})for(int flags:{0,8,32,64})for(int selector:{0,255})for(int offset:{-1000,0,500})
 for(int delta:{-100,0,100})for(int fraction:{0,128,255})for(int x:{-2000,-1500,-1000,-751,-750,-501,-500,-499,0,499,500,501,749,750,751,1000,2000}){
  rr::PathSample p{0,0,500,500,512,512};p.shoulder_left=p.shoulder_right=250;p.height_left=16;p.height_right=24;p.elevation=200;p.marking_flags=uint8_t(flags);
  auto np=p;np.left=520;np.right=480;np.height_left=20;np.height_right=40;
  rr::TerrainSlice t;t.type=uint8_t(kind);t.selectors={uint8_t(selector),uint8_t(selector)};t.walls[0]={100,900,100};t.walls[1]={80,1000,-1};
  for(int side=0;side<2;++side)for(int band=0;band<3;++band)t.points[size_t(band*2+side)]={(side?1:-1)*(1000+band*400),band==0?16:band==1?64:-32};
  auto nt=t;for(auto&v:nt.points){v[0]+=40;v[1]+=8;}
  const int height=rr::terrain_height(t,p,nt,np,x*256,uint8_t(fraction),delta,offset);
  if(!first){std::cout<<",";}first=false;std::cout<<"{\"id\":"<<id++<<",\"current\":";boundary(t,p,offset);
  std::cout<<",\"next\":";boundary(nt,np,offset+delta);std::cout<<",\"lateral\":"<<(offset+x)*256<<",\"fraction\":"<<fraction<<",\"height\":"<<height<<"}";
 }
 std::cout<<"],\"inventory\":[";first=true;int course_id=0;
 for(const char* stem:{"CITY","HIWAY","MEDLY","NAPA","CANYN"}){
  auto course=rr::read_course(assets.read(std::string("DATA/COURSES/")+stem+".CRS"));rr::CourseScene scene(assets,exe,course,stem,1,0);
  // Match the Rust renderer's documented private terrain stream. CourseScene
  // also places scenery between sections, so its later random terrain is a
  // different input; use the actual iterator with the isolated stream here.
  uint32_t terrain_seed=1;auto random=[&](){terrain_seed=terrain_seed*214013u+2531011u;return(terrain_seed>>16)&0x7fffu;};
  std::vector<std::vector<rr::TerrainSlice>> terrain;
  for(size_t section=0;section<course.sections.size();++section){
   const auto& data=course.sections[section];rr::TerrainIterator iterator(data.terrain,0,random);terrain.emplace_back();
   for(size_t local=0;local<data.samples.size();++local){
    if(local){iterator.step(256);}const auto&p=data.samples[local];auto t=iterator.slice(-p.left-p.shoulder_left,p.right+p.shoulder_right);
    if(t.type==4&&p.marking_flags&&!(p.marking_flags&0x40)){for(auto&point:t.points){point[1]=0;}}
    if(p.marking_flags&0x40){t.type=3;}
    if(t.type==0){t.selectors=scene.terrain(uint32_t(section),uint32_t(local)).selectors;}
    terrain.back().push_back(t);
   }
  }
  for(const auto& [node_id,node]:course.nodes){if(node.kind!=0)continue;
   for(uint32_t local=0;local<node.length;++local){const auto&p=course.sections[node.link].samples[local];const auto&t=terrain[node.link][local];
    if(!first){std::cout<<",";}first=false;std::cout<<"{\"course\":"<<course_id<<",\"node\":"<<node_id<<",\"local\":"<<local<<",\"boundary\":";
    boundary(t,p,0,p.event_type==5?scene.rail_height(node.link,local):0);
    rr::SliceEdges e{-p.left,p.right,-p.left-p.shoulder_left,p.right+p.shoulder_right,p.marking_flags,t.type};
    std::cout<<",\"classes\":[";bool first_class=true;
    for(int x:{-p.left-p.shoulder_left-1,-p.left-p.shoulder_left,-p.left-49,-p.left,-p.left+1,p.right-1,p.right,p.right+48,p.right+49,p.right+p.shoulder_right}){
     if(!first_class){std::cout<<",";}first_class=false;std::cout<<rr::surface_class(e,x*256);
    }
    std::cout<<"]}";
   }
  }++course_id;
 }
 std::cout<<"]}\n";
 }catch(const std::exception&e){std::cerr<<e.what()<<"\n";return 1;}}
