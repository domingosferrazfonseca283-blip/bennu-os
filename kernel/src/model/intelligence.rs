use super::sync::SpinLock;

pub const MAX_COGNITIVE_TASKS: usize = 16;
pub const MAX_COGNITIVE_OBSERVATIONS: usize = 32;
pub const MAX_COGNITIVE_MEMORY: usize = 64;
pub const MAX_COGNITIVE_PLAN_STEPS: usize = 8;
pub const MAX_COGNITIVE_STEP_DEPS: usize = 4;
pub const MAX_COGNITIVE_MODELS: usize = 4;
pub const MAX_COGNITIVE_RULES: usize = 32;

pub const MAX_COGNITIVE_GOALS: usize = 16;
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum GoalState { Empty=0, Pending=1, Active=2, AwaitingAuthorization=3, Achieved=4, Blocked=5, Failed=6 }
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CognitiveGoal { pub id:u64, pub task_id:u64, pub kind:TaskKind, pub priority:u8, pub capability_mask:u32, pub input:u64, pub state:GoalState, pub result:u64 }
impl CognitiveGoal { pub const EMPTY:Self=Self{id:0,task_id:0,kind:TaskKind::Observe,priority:0,capability_mask:0,input:0,state:GoalState::Empty,result:0}; }
static GOALS:SpinLock<[CognitiveGoal;MAX_COGNITIVE_GOALS]>=SpinLock::new([CognitiveGoal::EMPTY;MAX_COGNITIVE_GOALS]);
static GOAL_NEXT_ID:SpinLock<u64>=SpinLock::new(1);

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MemoryKind { Observation = 1, Decision = 2, Result = 3, Completion = 4 }
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CognitiveMemoryEntry { pub id:u64,pub kind:MemoryKind,pub task_id:u64,pub event_kind:u16,pub source:u64,pub value:u64,pub outcome:u64 }
impl CognitiveMemoryEntry { pub const EMPTY:Self=Self{id:0,kind:MemoryKind::Observation,task_id:0,event_kind:0,source:0,value:0,outcome:0}; }
static MEMORY:SpinLock<[CognitiveMemoryEntry;MAX_COGNITIVE_MEMORY]>=SpinLock::new([CognitiveMemoryEntry::EMPTY;MAX_COGNITIVE_MEMORY]);
static MEMORY_NEXT_ID:SpinLock<u64>=SpinLock::new(1);
pub fn remember(mut entry:CognitiveMemoryEntry)->u64{let mut n=MEMORY_NEXT_ID.lock();let id=*n.get();*n.get_mut()=id.wrapping_add(1).max(1);drop(n);entry.id=id;MEMORY.lock().get_mut()[((id-1)as usize)%MAX_COGNITIVE_MEMORY]=entry;id}
pub fn recall(id:u64)->Option<CognitiveMemoryEntry>{if id==0{return None} MEMORY.lock().get().iter().find(|e|e.id==id).copied()}
pub fn memory()->[CognitiveMemoryEntry;MAX_COGNITIVE_MEMORY]{*MEMORY.lock().get()}

#[repr(u8)]
#[derive(Clone,Copy,PartialEq,Eq)]
pub enum PlanStepState{Empty=0,Pending=1,Running=2,Completed=3,Failed=4,AwaitingAuthorization=5}
#[repr(C)]
#[derive(Clone,Copy)]
pub struct CognitivePlanStep{pub id:u64,pub task_id:u64,pub action:CognitiveDecision,pub capability_mask:u32,pub input:u64,pub output:u64,pub state:PlanStepState,pub dependency_count:u8,pub dependencies:[u64;MAX_COGNITIVE_STEP_DEPS]}
impl CognitivePlanStep{pub const EMPTY:Self=Self{id:0,task_id:0,action:CognitiveDecision::None,capability_mask:0,input:0,output:0,state:PlanStepState::Empty,dependency_count:0,dependencies:[0;MAX_COGNITIVE_STEP_DEPS]};}
static PLAN:SpinLock<[CognitivePlanStep;MAX_COGNITIVE_PLAN_STEPS]>=SpinLock::new([CognitivePlanStep::EMPTY;MAX_COGNITIVE_PLAN_STEPS]);
static PLAN_NEXT_ID:SpinLock<u64>=SpinLock::new(1);

#[repr(u8)]
#[derive(Clone,Copy,PartialEq,Eq)]
pub enum ModelKind{RuleBased=1,Statistical=2,Neural=3}
#[repr(C)]
#[derive(Clone,Copy)]
pub struct CognitiveModel{pub id:u64,pub kind:ModelKind,pub version:u32,pub memory_object:u64,pub bytes:u64,pub active:bool}
impl CognitiveModel{pub const EMPTY:Self=Self{id:0,kind:ModelKind::RuleBased,version:0,memory_object:0,bytes:0,active:false};}
static MODELS:SpinLock<[CognitiveModel;MAX_COGNITIVE_MODELS]>=SpinLock::new([CognitiveModel::EMPTY;MAX_COGNITIVE_MODELS]);
static MODEL_NEXT_ID:SpinLock<u64>=SpinLock::new(1);
pub fn register_model(kind:ModelKind,version:u32,memory_object:u64,bytes:u64)->Result<u64,&'static str>{if version==0||memory_object==0||bytes==0{return Err("invalid cognitive model")}let(_,pages)=super::runtime::object_frames(super::ObjectId(memory_object)).ok_or("cognitive model memory is not valid")?;if bytes>(pages as u64).checked_mul(crate::memory::PAGE_SIZE).ok_or("cognitive model backing size overflow")?{return Err("cognitive model exceeds backing memory")}let mut n=MODEL_NEXT_ID.lock();let id=*n.get();*n.get_mut()=id.wrapping_add(1).max(1);drop(n);let mut models=MODELS.lock();let has_active=models.get().iter().any(|m|m.active);for m in models.get_mut().iter_mut(){if !m.active{*m=CognitiveModel{id,kind,version,memory_object,bytes,active:!has_active};return Ok(id)}}Err("cognitive model registry full")}
pub fn activate_model(id:u64)->Result<(),&'static str>{let mut m=MODELS.lock();if !m.get().iter().any(|x|x.id==id&&x.id!=0){return Err("cognitive model not found")}for x in m.get_mut().iter_mut(){x.active=x.id==id}Ok(())}
pub fn active_model()->Option<CognitiveModel>{MODELS.lock().get().iter().find(|m|m.active).copied()}

#[repr(C)]
#[derive(Clone,Copy)]
pub struct CognitiveRule{pub id:u64,pub model_id:u64,pub event_kind:u16,pub task_kind:TaskKind,pub decision:CognitiveDecision,pub capability_mask:u32,pub priority:u8,pub enabled:bool}
impl CognitiveRule{pub const EMPTY:Self=Self{id:0,model_id:0,event_kind:0,task_kind:TaskKind::Observe,decision:CognitiveDecision::None,capability_mask:0,priority:0,enabled:false};}
static RULES:SpinLock<[CognitiveRule;MAX_COGNITIVE_RULES]>=SpinLock::new([CognitiveRule::EMPTY;MAX_COGNITIVE_RULES]);
static RULE_NEXT_ID:SpinLock<u64>=SpinLock::new(1);
pub fn register_rule(model_id:u64,event_kind:u16,task_kind:TaskKind,decision:CognitiveDecision,capability_mask:u32,priority:u8)->Result<u64,&'static str>{if model_id==0||event_kind==0||decision==CognitiveDecision::None||capability_mask&!CAPABILITY_ALL!=0{return Err("invalid cognitive rule")}if !MODELS.lock().get().iter().any(|m|m.id==model_id&&m.active){return Err("cognitive rule model not found")}let mut n=RULE_NEXT_ID.lock();let id=*n.get();*n.get_mut()=id.wrapping_add(1).max(1);drop(n);for r in RULES.lock().get_mut().iter_mut(){if !r.enabled{*r=CognitiveRule{id,model_id,event_kind,task_kind,decision,capability_mask,priority,enabled:true};return Ok(id)}}Err("cognitive rule registry full")}

#[repr(C)]
#[derive(Clone,Copy)]
pub struct CognitiveInference{pub model_id:u64,pub decision:CognitiveDecision,pub capability_mask:u32,pub confidence:u8,pub reason:u8,pub value:u64}
impl CognitiveInference{pub const EMPTY:Self=Self{model_id:0,decision:CognitiveDecision::None,capability_mask:0,confidence:0,reason:0,value:0};}

fn self_model_bias(task:&CognitiveTask)->(u8,u32){
    let entity=super::entity::state();
    let mut capability=0;
    if entity.resources.block_devices>0 { capability|=CAPABILITY_STORAGE; }
    if entity.resources.relations>0 { capability|=CAPABILITY_PROCESS; }
    let pressure=if entity.resources.objects>=64||entity.resources.cognitive_cycles>1_000_000{20}else{0};
    (pressure,capability)
}
fn rule_inference(model:CognitiveModel,task:&CognitiveTask)->Option<CognitiveInference>{
    let event_kind=(task.input>>48)as u16;let rules=RULES.lock();let mut selected:Option<CognitiveRule>=None;
    for r in rules.get().iter(){if r.enabled&&r.model_id==model.id&&r.event_kind==event_kind&&r.task_kind==task.kind&&(selected.is_none()||r.priority>selected.unwrap().priority){selected=Some(*r)}}
    selected.map(|r|{let(bias,cap)=self_model_bias(task);CognitiveInference{model_id:model.id,decision:r.decision,capability_mask:task.capability_mask|r.capability_mask|cap,confidence:100u8.saturating_sub(bias),reason:r.priority,value:task.input}})
}
fn infer_rule_based(model:CognitiveModel,task:&CognitiveTask)->CognitiveInference{
    if let Some(i)=rule_inference(model,task){return i}
    let event_kind=(task.input>>48)as u16;
    let decision=match task.kind{TaskKind::Observe=>CognitiveDecision::Observe,TaskKind::Execute if event_kind==super::EventKind::DeviceTransferCompleted as u16=>CognitiveDecision::Execute,TaskKind::Execute|TaskKind::Research|TaskKind::Communicate=>CognitiveDecision::RequestAuthorization};
    let(bias,cap)=self_model_bias(task);
    CognitiveInference{model_id:model.id,decision,capability_mask:task.capability_mask|cap,confidence:50u8.saturating_sub(bias),reason:0,value:task.input}
}
fn infer(task:&CognitiveTask)->Result<CognitiveInference,&'static str>{let m=active_model().ok_or("no active cognitive model")?;match m.kind{ModelKind::RuleBased=>Ok(infer_rule_based(m,task)),ModelKind::Statistical|ModelKind::Neural=>Err("cognitive model backend is not implemented")}}
pub fn plan_for_task(task_id:u64,kind:TaskKind,input:u64,capability_mask:u32)->Result<u64,&'static str>{let mut t=CognitiveTask::EMPTY;t.id=task_id;t.kind=kind;t.input=input;t.capability_mask=capability_mask;let i=infer(&t)?;let mut n=PLAN_NEXT_ID.lock();let id=*n.get();*n.get_mut()=id.wrapping_add(1).max(1);drop(n);for s in PLAN.lock().get_mut().iter_mut(){if matches!(s.state,PlanStepState::Empty|PlanStepState::Completed|PlanStepState::Failed){*s=CognitivePlanStep{id,task_id,action:i.decision,capability_mask:i.capability_mask,input,output:i.value,state:if i.decision==CognitiveDecision::RequestAuthorization{PlanStepState::AwaitingAuthorization}else{PlanStepState::Pending},dependency_count:0,dependencies:[0;MAX_COGNITIVE_STEP_DEPS]};return Ok(id)}}Err("cognitive plan full")}
pub fn plan_step(id:u64)->Option<CognitivePlanStep>{PLAN.lock().get().iter().find(|s|s.id==id).copied()}
pub fn plan_step_ready(id:u64)->bool{let p=PLAN.lock();let s=match p.get().iter().find(|s|s.id==id){Some(v)=>v,None=>return false};if s.state!=PlanStepState::Pending{return false}for i in 0..s.dependency_count as usize{let d=s.dependencies[i];match p.get().iter().find(|x|x.id==d){Some(x) if x.state==PlanStepState::Completed=>{},_=>return false}}true}
pub fn complete_plan_step(id:u64,output:u64)->Result<(),&'static str>{let mut p=PLAN.lock();let s=p.get_mut().iter_mut().find(|s|s.id==id).ok_or("cognitive plan step not found")?;if s.state!=PlanStepState::Running{return Err("cognitive plan step is not running")}s.output=output;s.state=PlanStepState::Completed;Ok(())}

pub const CAPABILITY_OBSERVE:u32=1<<0;pub const CAPABILITY_RESEARCH:u32=1<<1;pub const CAPABILITY_EXECUTE:u32=1<<2;pub const CAPABILITY_COMMUNICATE:u32=1<<3;pub const CAPABILITY_STORAGE:u32=1<<4;pub const CAPABILITY_NETWORK:u32=1<<5;pub const CAPABILITY_PROCESS:u32=1<<6;pub const CAPABILITY_ALL:u32=CAPABILITY_OBSERVE|CAPABILITY_RESEARCH|CAPABILITY_EXECUTE|CAPABILITY_COMMUNICATE|CAPABILITY_STORAGE|CAPABILITY_NETWORK|CAPABILITY_PROCESS;
#[repr(u8)]#[derive(Clone,Copy,PartialEq,Eq)]pub enum TaskKind{Observe=1,Research=2,Execute=3,Communicate=4}
#[repr(u8)]#[derive(Clone,Copy,PartialEq,Eq)]pub enum TaskState{Empty=0,Queued=1,Planning=2,AwaitingAuthorization=3,Running=4,Verifying=5,Completed=6,Failed=7}
#[repr(u8)]#[derive(Clone,Copy,PartialEq,Eq)]pub enum CognitivePhase{Idle=0,Observe=1,Plan=2,Execute=3,Verify=4,Communicate=5}
#[repr(C)]#[derive(Clone,Copy)]pub struct CognitiveObservation{pub event_kind:u16,pub source:u64,pub target:u64,pub value:u64}
impl CognitiveObservation{pub const EMPTY:Self=Self{event_kind:0,source:0,target:0,value:0};}
#[repr(C)]#[derive(Clone,Copy)]pub struct AuthorizationRequest{pub id:u64,pub task_id:u64,pub kind:TaskKind,pub capability_mask:u32,pub input:u64,pub source:u64}
impl AuthorizationRequest{pub const EMPTY:Self=Self{id:0,task_id:0,kind:TaskKind::Observe,capability_mask:0,input:0,source:0};}
#[repr(u8)]#[derive(Clone,Copy,PartialEq,Eq)]pub enum CognitiveDecision{None=0,Observe=1,RequestAuthorization=2,Execute=3,Verify=4,Remember=5}
#[repr(C)]#[derive(Clone,Copy)]pub struct CognitiveTask{pub id:u64,pub kind:TaskKind,pub state:TaskState,pub phase:CognitivePhase,pub priority:u8,pub capability_mask:u32,pub input:u64,pub source:u64,pub output:u64,pub decision:CognitiveDecision,pub authorized:bool}
impl CognitiveTask{pub const EMPTY:Self=Self{id:0,kind:TaskKind::Observe,state:TaskState::Empty,phase:CognitivePhase::Idle,priority:0,capability_mask:0,input:0,source:0,output:0,decision:CognitiveDecision::None,authorized:false};}
#[repr(C)]#[derive(Clone,Copy)]pub struct CognitiveRuntime{pub next_id:u64,pub active:u64,pub phase:CognitivePhase,pub cycles:u64,pub completed:u64,pub failed:u64}
impl CognitiveRuntime{pub const EMPTY:Self=Self{next_id:1,active:0,phase:CognitivePhase::Idle,cycles:0,completed:0,failed:0};}
static TASKS:SpinLock<[CognitiveTask;MAX_COGNITIVE_TASKS]>=SpinLock::new([CognitiveTask::EMPTY;MAX_COGNITIVE_TASKS]);
static OBSERVATIONS:SpinLock<[CognitiveObservation;MAX_COGNITIVE_OBSERVATIONS]>=SpinLock::new([CognitiveObservation::EMPTY;MAX_COGNITIVE_OBSERVATIONS]);
static RUNTIME:SpinLock<CognitiveRuntime>=SpinLock::new(CognitiveRuntime::EMPTY);

pub fn init(){*GOALS.lock().get_mut()=[CognitiveGoal::EMPTY;MAX_COGNITIVE_GOALS];*GOAL_NEXT_ID.lock().get_mut()=1;*RUNTIME.lock().get_mut()=CognitiveRuntime::EMPTY;*TASKS.lock().get_mut()=[CognitiveTask::EMPTY;MAX_COGNITIVE_TASKS];*OBSERVATIONS.lock().get_mut()=[CognitiveObservation::EMPTY;MAX_COGNITIVE_OBSERVATIONS];*MEMORY.lock().get_mut()=[CognitiveMemoryEntry::EMPTY;MAX_COGNITIVE_MEMORY];*MEMORY_NEXT_ID.lock().get_mut()=1;*PLAN.lock().get_mut()=[CognitivePlanStep::EMPTY;MAX_COGNITIVE_PLAN_STEPS];*PLAN_NEXT_ID.lock().get_mut()=1;*MODELS.lock().get_mut()=[CognitiveModel::EMPTY;MAX_COGNITIVE_MODELS];*MODEL_NEXT_ID.lock().get_mut()=1;*RULES.lock().get_mut()=[CognitiveRule::EMPTY;MAX_COGNITIVE_RULES];*RULE_NEXT_ID.lock().get_mut()=1;}

pub fn observe(event:super::Event)->Result<(),&'static str>{let mut o=OBSERVATIONS.lock();for s in o.get_mut().iter_mut(){if s.event_kind==0{*s=CognitiveObservation{event_kind:event.kind as u16,source:event.source.0,target:event.target.0,value:event.value};remember(CognitiveMemoryEntry{id:0,kind:MemoryKind::Observation,task_id:0,event_kind:event.kind as u16,source:event.source.0,value:event.value,outcome:event.target.0});return Ok(())}}Err("cognitive observation queue full")}
fn consume_observation()->Option<CognitiveObservation>{let mut o=OBSERVATIONS.lock();let s=o.get_mut().iter_mut().find(|s|s.event_kind!=0)?;let v=*s;*s=CognitiveObservation::EMPTY;Some(v)}
fn observation_task_kind(o:CognitiveObservation)->TaskKind{match o.event_kind{5|6|8|9|10|11=>TaskKind::Execute,_=>TaskKind::Observe}}
fn kind_capability(k:TaskKind)->u32{match k{TaskKind::Observe=>CAPABILITY_OBSERVE,TaskKind::Research=>CAPABILITY_RESEARCH,TaskKind::Execute=>CAPABILITY_EXECUTE,TaskKind::Communicate=>CAPABILITY_COMMUNICATE}}

pub fn submit(kind:TaskKind,priority:u8,capability_mask:u32,input:u64)->Result<u64,&'static str>{let requested=capability_mask|kind_capability(kind);if requested&!CAPABILITY_ALL!=0{return Err("unknown cognitive capability")}let mut r=RUNTIME.lock();let rs=r.get_mut();let id=rs.next_id;rs.next_id=rs.next_id.wrapping_add(1).max(1);drop(r);for t in TASKS.lock().get_mut().iter_mut(){if matches!(t.state,TaskState::Empty|TaskState::Completed|TaskState::Failed){*t=CognitiveTask{id,kind,state:TaskState::Queued,phase:CognitivePhase::Idle,priority,capability_mask:requested,input,source:0,output:0,decision:CognitiveDecision::None,authorized:false};return Ok(id)}}Err("cognitive task queue full")}

pub fn submit_goal(kind:TaskKind,priority:u8,capability_mask:u32,input:u64)->Result<u64,&'static str>{
    if !GOALS.lock().get().iter().any(|g|g.state==GoalState::Empty||g.state==GoalState::Achieved||g.state==GoalState::Failed){return Err("cognitive goal registry full")}
    let task_id=submit(kind,priority,capability_mask,input)?;
    let mut next=GOAL_NEXT_ID.lock(); let id=*next.get(); *next.get_mut()=id.wrapping_add(1).max(1); drop(next);
    let mut goals=GOALS.lock();
    for goal in goals.get_mut().iter_mut(){
        if goal.state==GoalState::Empty || goal.state==GoalState::Achieved || goal.state==GoalState::Failed {
            *goal=CognitiveGoal{id,task_id,kind,priority,capability_mask:capability_mask|kind_capability(kind),input,state:GoalState::Pending,result:0};
            return Ok(id);
        }
    }
    Err("cognitive goal registry full")
}
pub fn goal(id:u64)->Option<CognitiveGoal>{GOALS.lock().get().iter().find(|g|g.id==id).copied()}
pub fn goals()->[CognitiveGoal;MAX_COGNITIVE_GOALS]{*GOALS.lock().get()}
fn update_goal_for_task(task:CognitiveTask,state:GoalState,result:u64){let mut goals=GOALS.lock();for g in goals.get_mut().iter_mut(){if g.task_id==task.id && g.state!=GoalState::Empty{g.state=state;g.result=result;break;}}}
pub fn goal_for_task(task_id:u64)->Option<CognitiveGoal>{GOALS.lock().get().iter().find(|g|g.task_id==task_id&&g.state!=GoalState::Empty).copied()}
fn activate_goal_for_task(task:CognitiveTask){let mut goals=GOALS.lock();for g in goals.get_mut().iter_mut(){if g.task_id==task.id&&g.state==GoalState::Pending{g.state=GoalState::Active;break;}}}

pub fn authorization_request(id:u64)->Option<AuthorizationRequest>{let t=TASKS.lock();let x=t.get().iter().find(|x|x.id==id&&x.state==TaskState::AwaitingAuthorization)?;Some(AuthorizationRequest{id:x.id,task_id:x.id,kind:x.kind,capability_mask:x.capability_mask,input:x.input,source:x.source})}
pub fn authorize_task(id:u64)->Result<(),&'static str>{let mut t=TASKS.lock();let x=t.get_mut().iter_mut().find(|x|x.id==id).ok_or("cognitive task not found")?;if x.state!=TaskState::AwaitingAuthorization{return Err("cognitive task is not awaiting authorization")}x.authorized=true;x.state=TaskState::Queued;update_goal_for_task(*x,GoalState::Active,0);let _=plan_for_task(x.id,x.kind,x.input,x.capability_mask);Ok(())}
pub fn deny_task(id:u64)->Result<(),&'static str>{let mut t=TASKS.lock();let x=t.get_mut().iter_mut().find(|x|x.id==id).ok_or("cognitive task not found")?;if x.state!=TaskState::AwaitingAuthorization{return Err("cognitive task is not awaiting authorization")}x.state=TaskState::Failed;x.authorized=false;update_goal_for_task(*x,GoalState::Blocked,0);remember(CognitiveMemoryEntry{id:0,kind:MemoryKind::Decision,task_id:x.id,event_kind:(x.input>>48)as u16,source:x.source,value:x.input,outcome:0});Ok(())}
fn select_task(t:&[CognitiveTask;MAX_COGNITIVE_TASKS])->Option<usize>{let mut s=None;let mut p=0;for(i,x)in t.iter().enumerate(){if x.state==TaskState::Queued&&(s.is_none()||x.priority>p){s=Some(i);p=x.priority}}s}

pub fn tick(){let mut r=RUNTIME.lock();let rs=r.get_mut();rs.cycles=rs.cycles.wrapping_add(1);let mut tasks=TASKS.lock();let i=match select_task(tasks.get()){Some(v)=>v,None=>{drop(tasks);if let Some(o)=consume_observation(){let k=observation_task_kind(o);let input=((o.event_kind as u64)<<48)|(o.value&0x0000_ffff_ffff_ffff);drop(r);let _=submit(k,0,0,input).map(|id|{if let Some(t)=TASKS.lock().get_mut().iter_mut().find(|t|t.id==id){t.source=o.source}});return}rs.active=0;rs.phase=CognitivePhase::Idle;return}};let t=&mut tasks.get_mut()[i];activate_goal_for_task(*t);rs.active=t.id;match t.phase{CognitivePhase::Idle=>{t.phase=CognitivePhase::Observe;t.state=TaskState::Planning;rs.phase=CognitivePhase::Observe},CognitivePhase::Observe=>{t.phase=CognitivePhase::Plan;rs.phase=CognitivePhase::Plan},CognitivePhase::Plan=>{if !t.authorized{t.decision=CognitiveDecision::RequestAuthorization;t.state=TaskState::AwaitingAuthorization;update_goal_for_task(*t,GoalState::AwaitingAuthorization,0);let _=plan_for_task(t.id,t.kind,t.input,t.capability_mask);remember(CognitiveMemoryEntry{id:0,kind:MemoryKind::Decision,task_id:t.id,event_kind:(t.input>>48)as u16,source:t.source,value:t.input,outcome:CognitiveDecision::RequestAuthorization as u64});rs.phase=CognitivePhase::Plan}else{match infer(t){Ok(i)=>{t.decision=i.decision;let _=plan_for_task(t.id,t.kind,t.input,i.capability_mask);t.phase=if i.decision==CognitiveDecision::Execute{CognitivePhase::Execute}else{CognitivePhase::Communicate};t.state=TaskState::Running;remember(CognitiveMemoryEntry{id:0,kind:MemoryKind::Decision,task_id:t.id,event_kind:(t.input>>48)as u16,source:t.source,value:i.value,outcome:i.decision as u64});rs.phase=t.phase},Err(_)=>{t.state=TaskState::Failed;t.phase=CognitivePhase::Idle;update_goal_for_task(*t,GoalState::Failed,0);rs.failed=rs.failed.wrapping_add(1)}}}},CognitivePhase::Execute=>{},CognitivePhase::Verify=>{t.phase=CognitivePhase::Communicate;rs.phase=CognitivePhase::Communicate},CognitivePhase::Communicate=>{t.state=TaskState::Completed;t.phase=CognitivePhase::Idle;remember(CognitiveMemoryEntry{id:0,kind:MemoryKind::Completion,task_id:t.id,event_kind:(t.input>>48)as u16,source:t.source,value:t.output,outcome:1});rs.completed=rs.completed.wrapping_add(1);rs.phase=CognitivePhase::Idle;rs.active=0}}}

pub fn execute_authorized(){let pending={let t=TASKS.lock();t.get().iter().find(|t|t.state==TaskState::Running&&t.authorized).copied()};let task=match pending{Some(v)=>v,None=>return};let result=match task.kind{TaskKind::Execute if task.input>>48==super::EventKind::DeviceTransferCompleted as u64=>super::runtime::prepare_bennufs_mount(super::ObjectId(task.source)),TaskKind::Observe=>Ok(()),_=>Err("cognitive action has no kernel executor")};let mut t=TASKS.lock();if let Some(c)=t.get_mut().iter_mut().find(|c|c.id==task.id){match result{Ok(())=>{c.output=1;c.decision=CognitiveDecision::Verify;c.phase=CognitivePhase::Verify;c.state=TaskState::Verifying;update_goal_for_task(*c,GoalState::Achieved,1);remember(CognitiveMemoryEntry{id:0,kind:MemoryKind::Result,task_id:c.id,event_kind:(c.input>>48)as u16,source:c.source,value:c.input,outcome:1})},Err(_)=>{c.output=0;c.state=TaskState::Failed;c.phase=CognitivePhase::Idle;update_goal_for_task(*c,GoalState::Failed,0);remember(CognitiveMemoryEntry{id:0,kind:MemoryKind::Result,task_id:c.id,event_kind:(c.input>>48)as u16,source:c.source,value:c.input,outcome:0})}}}}
pub fn runtime()->CognitiveRuntime{*RUNTIME.lock().get()}
pub fn task(id:u64)->Option<CognitiveTask>{TASKS.lock().get().iter().find(|t|t.id==id).copied()}