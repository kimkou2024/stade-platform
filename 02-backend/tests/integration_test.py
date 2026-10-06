import requests, jwt, pyotp, time, uuid
B="http://127.0.0.1:8080"; SEC="integration-test-secret-key-at-least-32b"
def jreq(m,p,tok=None,**kw):
    h={"Authorization":f"Bearer {tok}"} if tok else {}
    r=requests.request(m,B+p,headers=h,timeout=10,**kw)
    try: j=r.json()
    except Exception: j=r.text
    return r.status_code,j
ok=0; fail=0
def chk(name,cond,extra=""):
    global ok,fail; ok+=bool(cond); fail+=(not cond)
    print(("PASS" if cond else "FAIL"),"-",name,"" if cond else f"<< {extra}")

email=f"k_{uuid.uuid4().hex[:6]}@dz.test"
s,j=jreq("POST","/api/auth/register",json={"email":email,"password":"supersecret","full_name":"Karim Test","consent":True})
chk("register",s==200 and "id" in j,(s,j)); uid=j["id"]
s,j=jreq("POST","/api/auth/login",json={"email":email,"password":"supersecret"})
chk("login",s==200 and "access_token" in j,(s,j)); utok=j["access_token"]; rtok=j["refresh_token"]
s,j=jreq("GET","/api/me",utok); chk("me",s==200 and j.get("email")==email,(s,j))
s,j=jreq("POST","/api/auth/refresh",json={"refresh_token":rtok}); chk("refresh",s==200 and "access_token" in j,(s,j))
s,j=jreq("GET","/api/me",rtok); chk("refresh-token-rejected-as-access",s==401,(s,j))
# 2FA
s,j=jreq("POST","/api/auth/2fa/setup",utok); chk("2fa setup",s==200 and "secret" in j,(s,j)); secret=j.get("secret")
code=pyotp.TOTP(secret).now()
s,j=jreq("POST","/api/auth/2fa/verify",utok,json={"code":code}); chk("2fa verify",s==200 and j.get("twofa_enabled"),(s,j))
# login now requires code
s,j=jreq("POST","/api/auth/login",json={"email":email,"password":"supersecret"}); chk("login blocked w/o 2fa",s==400,(s,j))
s,j=jreq("POST","/api/auth/login",json={"email":email,"password":"supersecret","totp_code":pyotp.TOTP(secret).now()}); chk("login with 2fa",s==200,(s,j))
# staff token (same uid so FKs hold)
stok=jwt.encode({"sub":uid,"role":"admin","token_type":"access","iat":int(time.time()),"exp":int(time.time())+3600},SEC,algorithm="HS256")
# zones
s,zj=jreq("GET","/api/zones"); chk("list zones",s==200 and len(zj)==8,(s,len(zj) if isinstance(zj,list) else zj))
zone_z1=[z for z in zj if z["code"]=="Z1"][0]["id"]
# admin: create event
s,j=jreq("POST","/api/admin/events",stok,json={"title":"USMA vs MCA","starts_at":"2026-11-01T18:00:00Z","capacity":38000}); chk("create event",s==200,(s,j)); eid=j.get("id")
s,j=jreq("POST","/api/admin/events/%s/zones"%eid,stok,json={"zone_id":zone_z1,"quota":100,"price_dzd":"500.00","ticket_category":"local"}); chk("configure zone",s==200,(s,j))
s,j=jreq("PATCH","/api/admin/events/%s/status"%eid,stok,json={"status":"sales_open"}); chk("open sales",s==200,(s,j))
# availability
s,j=jreq("GET","/api/events/%s/zones"%eid); av=j["zones"][0]["available"] if s==200 and j.get("zones") else None
chk("availability=100",av==100,(s,j))
# user flow: seat-lock -> order -> pay
s,j=jreq("POST","/api/events/%s/seat-lock"%eid,utok,json={"zone_id":zone_z1,"qty":2}); chk("seat-lock",s==200 and "lock_id" in j,(s,j)); lock=j.get("lock_id")
s,j=jreq("POST","/api/orders",utok,json={"lock_id":lock,"holder_names":["Karim","Nacer"]}); chk("create order",s==200 and j.get("total_dzd")=="1000.00",(s,j)); oid=j.get("order_id")
s,j=jreq("POST","/api/orders/%s/pay"%oid,utok,json={"lock_id":lock,"holder_names":["Karim","Nacer"],"gateway":"cib"}); chk("pay+issue tickets",s==200 and len(j.get("tickets",[]))==2,(s,j))
qr0=j["tickets"][0]["qr"]; tid0=j["tickets"][0]["ticket_id"]
# availability now 98
s,j=jreq("GET","/api/events/%s/zones"%eid); av=j["zones"][0]["available"]; chk("availability=98 after sale",av==98,(s,av))
# over-limit lock rejected (qty 6 > max 5)
s,j=jreq("POST","/api/events/%s/seat-lock"%eid,utok,json={"zone_id":zone_z1,"qty":6}); chk("qty>5 rejected",s==400,(s,j))
# access: public key + manifest + scan
s,j=jreq("GET","/api/access/public-key"); chk("public key",s==200 and j.get("alg")=="ed25519",(s,j))
s,j=jreq("GET","/api/access/events/%s/manifest"%eid,stok); chk("manifest has 2 tickets",s==200 and j.get("count")==2,(s,j))
s,j=jreq("POST","/api/access/scan",stok,json={"token":qr0,"gate_code":"V19","device_id":"PDA-1"}); chk("scan valid",s==200 and j.get("result")=="valid",(s,j))
s,j=jreq("POST","/api/access/scan",stok,json={"token":qr0,"gate_code":"V19","device_id":"PDA-1"}); chk("re-scan=used",s==200 and j.get("result")=="used",(s,j))
s,j=jreq("POST","/api/access/scan",stok,json={"token":"garbage.sig","gate_code":"V19","device_id":"PDA-1"}); chk("bad qr=invalid",s==200 and j.get("result")=="invalid",(s,j))
# offline sync duplicate -> conflict (ticket already used)
s,j=jreq("POST","/api/access/sync",stok,json={"entries":[{"ticket_id":tid0,"event_id":eid,"gate_code":"V20","device_id":"PDA-2","scanned_at":"2026-11-01T18:05:00Z"}]}); chk("sync duplicate->conflict",s==200 and j.get("conflicts")==1,(s,j))
# reporting
s,j=jreq("GET","/api/admin/events/%s/reports/sales"%eid,stok); chk("sales report",s==200 and j.get("total_sold")==2,(s,j))
s,j=jreq("GET","/api/admin/events/%s/dashboard"%eid,stok); chk("dashboard entries=1",s==200 and j.get("entries")==1,(s,j))
print(f"\n==== {ok} passed, {fail} failed ====")

# ---- RBAC bootstrap + real staff login (added) ----
import os as _os
_BT = _os.environ.get("BOOTSTRAP_TOKEN", "")
_r = requests.post(B+"/api/admin/bootstrap", headers={"X-Bootstrap-Token": _BT, "content-type":"application/json"},
                   json={"email": email}, timeout=10)
chk("bootstrap promotes to admin", _r.status_code==200 and _r.json().get("role")=="admin", (_r.status_code, _r.text))
_r = requests.post(B+"/api/admin/bootstrap", headers={"X-Bootstrap-Token":"wrong","content-type":"application/json"},
                   json={"email": email}, timeout=10)
chk("bootstrap rejects bad token", _r.status_code==401, (_r.status_code,))
# login again (2FA enabled earlier) and confirm the JWT now carries role=admin
s,j=jreq("POST","/api/auth/login",json={"email":email,"password":"supersecret","totp_code":pyotp.TOTP(secret).now()})
ok_role=False
if s==200:
    claims=jwt.decode(j["access_token"], SEC, algorithms=["HS256"]); ok_role=claims.get("role")=="admin"
chk("login now returns role=admin", ok_role, (s,j))
# that real admin token can hit a staff endpoint
if s==200:
    s2,j2=jreq("POST","/api/admin/events", j["access_token"], json={"title":"Admin UI test","starts_at":"2026-12-01T18:00:00Z"})
    chk("real admin token creates event", s2==200, (s2,j2))
print(f"\n==== FINAL {ok} passed, {fail} failed ====")

# ---- VIP invitations, subscriptions, exports, audit (added) ----
# uses the real admin token from the previous block via a fresh login
_tok = None
s,j=jreq("POST","/api/auth/login",json={"email":email,"password":"supersecret","totp_code":pyotp.TOTP(secret).now()})
if s==200: _tok=j["access_token"]
# VIP invitation create + scan
s,j=jreq("POST","/api/admin/events/%s/invitations"%eid,_tok,json={"tribune":"A","holder_name":"VIP Guest","second_check_required":True})
chk("vip invitation created", s==200 and "qr" in j, (s,j)); inv_qr=j.get("qr")
s,j=jreq("GET","/api/admin/events/%s/invitations"%eid,_tok)
chk("vip invitations list", s==200 and len(j.get("invitations",[]))>=1, (s,j))
s,j=jreq("POST","/api/access/invitation-scan",_tok,json={"token":inv_qr,"gate_code":"VIP","device_id":"PDA-9"})
chk("vip invitation scan valid", s==200 and j.get("result")=="valid", (s,j))
s,j=jreq("POST","/api/access/invitation-scan",_tok,json={"token":inv_qr,"gate_code":"VIP","device_id":"PDA-9"})
chk("vip invitation re-scan used", s==200 and j.get("result")=="used", (s,j))
# subscription
s,j=jreq("POST","/api/admin/subscriptions",_tok,json={"email":email,"season":"2026-27","zone_code":"Z1"})
chk("subscription created", s==200 and j.get("card_ref","").startswith("SUB-"), (s,j))
s,j=jreq("GET","/api/admin/subscriptions",_tok)
chk("subscriptions list", s==200 and len(j.get("subscriptions",[]))>=1, (s,j))
# CSV export (raw request to read text)
_r=requests.get(B+"/api/admin/events/%s/exports/spectators"%eid, headers={"Authorization":f"Bearer {_tok}"}, timeout=10)
chk("spectator CSV export", _r.status_code==200 and _r.text.startswith("ticket_id,holder_name,zone,status"), (_r.status_code, _r.text[:60]))
# audit log has entries (event.create / event.status were recorded)
s,j=jreq("GET","/api/admin/audit-log",_tok)
chk("audit log populated", s==200 and len(j.get("entries",[]))>=1, (s,j))

# ---- password reset, PDF export, redis, notifications (added) ----
import uuid as _uuid
e2=f"r_{_uuid.uuid4().hex[:6]}@dz.test"
s,j=jreq("POST","/api/auth/register",json={"email":e2,"password":"supersecret","full_name":"Reset User","consent":True})
chk("register w/ consent", s==200, (s,j))
s,j=jreq("POST","/api/auth/register",json={"email":"noconsent@dz.test","password":"supersecret","full_name":"X"})
chk("register without consent rejected", s==400, (s,j))
s,j=jreq("POST","/api/auth/password/forgot",json={"email":e2})
chk("password forgot returns dev_token", s==200 and j.get("dev_token"), (s,j)); rt=j.get("dev_token")
s,j=jreq("POST","/api/auth/password/reset",json={"token":rt,"new_password":"brandnewpass"})
chk("password reset ok", s==200 and j.get("ok"), (s,j))
s,j=jreq("POST","/api/auth/login",json={"email":e2,"password":"brandnewpass"})
chk("login with new password", s==200 and "access_token" in j, (s,j))
# PDF spectator export
_r=requests.get(B+"/api/admin/events/%s/exports/spectators?format=pdf"%eid, headers={"Authorization":f"Bearer {_tok}"}, timeout=10)
chk("spectator PDF export", _r.status_code==200 and _r.content[:4]==b"%PDF", (_r.status_code, _r.content[:8]))
# health shows redis
import json as _json
_h=requests.get(B+"/health",timeout=10).json()
chk("health reports redis", _h.get("redis")==True, _h)
# notifications recorded for main user (purchase confirmation)
s,j=jreq("GET","/api/me/notifications", utok)
chk("notifications recorded", s==200 and len(j.get("notifications",[]))>=1, (s,j))
print(f"\n==== GRAND TOTAL {ok} passed, {fail} failed ====")

# ---- retention purge (added) ----
s,j=jreq("POST","/api/admin/retention/purge",_tok,json={"days":365})
chk("retention purge", s==200 and "purged" in j, (s,j))
print(f"\n==== END {ok} passed, {fail} failed ====")
