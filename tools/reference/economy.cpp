// Independently authored public-API probe, pinned reference; metadata output stays private.
#include "career.hpp"
#include "assets.hpp"
#include <iostream>
#include <algorithm>
int main(int argc,char**argv){try{
 if(argc!=2){return 2;}
 rr::Assets assets(argv[1]);auto exe=assets.read("ROADRASH.EXE");auto t=rr::read_career_tables(exe);
 std::cout<<"{\"characters\":[";
 for(int id=1;id<=8;++id){if(id!=1)std::cout<<",";int r=rr::character_rider(id);std::cout<<"["<<id<<","<<r<<","<<t.start_cash[size_t(id)]<<",[";for(int l=0;l<5;++l){if(l)std::cout<<",";std::cout<<int(t.level_bike[size_t(r*5+l)]);}std::cout<<"]]";}
 std::cout<<"],\"prices\":[";for(size_t i=0;i<15;++i){if(i)std::cout<<",";std::cout<<t.bike_price[i];}
 std::cout<<"],\"prizes\":[";for(size_t i=0;i<15;++i){if(i)std::cout<<",";std::cout<<t.prize[i];}
 std::cout<<"],\"purchases\":[";bool first=true;
 for(int current=0;current<15;++current)for(int target=0;target<15;++target){int64_t needed=std::max<int64_t>(0,int64_t(t.bike_price[size_t(target)])-int64_t(t.bike_price[size_t(current)]/2));for(int64_t cash:{int64_t(0),std::max<int64_t>(0,needed-1),needed,needed+1}){
 rr::Career c;c.state.bike=uint8_t(current);c.state.cash=uint32_t(cash);auto status=rr::buy_bike(c,t,target);int code=status==rr::PurchaseResult::bought?0:status==rr::PurchaseResult::already_owned?1:2;if(!first)std::cout<<",";first=false;std::cout<<"["<<code<<","<<c.state.cash<<","<<int(c.state.bike)<<"]";}}
 std::cout<<"],\"outcomes\":[";first=true;
 for(int level=1;level<=5;++level)for(int kind=0;kind<3;++kind)for(int place=0;place<15;++place)for(int cash:{0,400,1000000}){
 rr::Career c;c.mode=rr::GameMode::big_game;c.state.character=1;c.state.level=uint8_t(level-1);c.state.course=1;c.state.bike=0;c.state.cash=uint32_t(cash);
 auto outcome=kind==0?(place<3?rr::RaceOutcome::won:rr::RaceOutcome::lost):kind==1?rr::RaceOutcome::busted:rr::RaceOutcome::wrecked;
 auto r=rr::finish_race(c,t,outcome,place);if(!first)std::cout<<",";first=false;std::cout<<"["<<r.prize<<","<<r.penalty<<","<<(r.game_over?"true":"false")<<","<<c.state.cash<<","<<int(c.state.qualified)<<","<<int(c.state.bike)<<","<<int(c.state.level+1)<<"]";
 }
 std::cout<<"]}\n";
}catch(std::exception&e){std::cerr<<e.what()<<"\n";return 1;}}
