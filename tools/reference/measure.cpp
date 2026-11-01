#include "actor_nodes.hpp"
#include "course_scene.hpp"
#include "world_renderer.hpp"
#include "view.hpp"
#include <fstream>
#include <iostream>
int main(int argc,char**argv){
 try {
  if(argc!=5)return 2;
  rr::Assets assets(argv[1]);auto exe=assets.read("ROADRASH.EXE");auto depths=rr::read_depth_table(exe);
  auto course=rr::read_course(assets.read(std::string("DATA/COURSES/")+argv[2]+".CRS"));
  auto route=rr::make_route(course,false);auto position=uint32_t(std::stoul(argv[3])*256);
  rr::Camera camera{0,150};auto slices=rr::project_road(route,position,camera,depths);
  rr::CourseScene scene(assets,exe,course,argv[2],1,0);
  rr::ActorNodes actors(assets,exe,true);auto frame=actors.library().graphics[0].frames.at(2);
  auto &near=slices.at(1).road[slices.at(1).own];auto &far=slices.at(2).road[slices.at(2).own];
  rr::ActorProjection p;p.depth=slices.at(1).depth;p.road_x=near.road_x;p.far_x=far.road_x;p.road_y=near.world_y;p.far_y=far.world_y;
  auto rider=rr::actor_node(frame,actors.texture(0,2,0),p,camera,depths);
  auto nodes=scene.draw_nodes(route,slices,camera,depths);if(rider)nodes.push_back(*rider);
  rr::Framebuffer fb(640,480);fb.clear(0);scene.draw_horizon(fb,0);rr::render_nodes(fb,nodes,rr::view_mode(true),0);
  std::vector<uint8_t> rgba(640*480*4);fb.to_rgba(rr::Palette::from_raw(assets.read("DATA/PALETTE.RAW")),rgba.data(),640*4);
  std::ofstream image(argv[4],std::ios::binary);image<<"P6\n640 480\n255\n";for(size_t i=0;i<rgba.size();i+=4)image.write(reinterpret_cast<const char*>(rgba.data()+i),3);
  std::cout<<"{\"course\":\""<<argv[2]<<"\",\"sample\":"<<position/256<<",\"rider_logical\":["<<frame.width<<","<<frame.height<<"],\"rider_hotspot\":["<<frame.hotspot[0]<<","<<frame.hotspot[1]<<"],\"actor_depth\":"<<p.depth<<",\"bounds\":[";
  if(!rider)return 3;std::cout<<rider->view_points[0].x*2./65536<<","<<rider->view_points[0].y*2./65536<<","<<rider->view_points[2].x*2./65536<<","<<rider->view_points[2].y*2./65536<<"],\"road\":[";
  for(size_t i=0;i<slices.size();++i){auto&s=slices[i];auto&r=s.road[s.own];if(i)std::cout<<",";std::cout<<"{\"z\":"<<s.depth<<",\"scale\":"<<s.scale*2./65536<<",\"x\":"<<r.road_x<<",\"y\":"<<r.world_y<<",\"screen_y\":"<<r.screen_y*2./65536<<",\"left\":"<<r.left<<",\"right\":"<<r.right<<"}";}std::cout<<"]}\n";
 }catch(std::exception&e){std::cerr<<e.what()<<"\n";return 1;}
}
