import socketio
from aiohttp import web

server_io = socketio.AsyncServer()

# app = socketio.ASGIApp(server_io, static_files={
#     '/': '/client/index.html', 
#     '/index.js': '/client/index.js'
# })
app = web.Application()
server_io.attach(app)

# a Python dictionary comprised of some heroes and their names
dict_names = {
  "Alison": "Zhang", 
  "Harry": "Yu", 
  "Mark": "Wang"
}

async def index(request):
    """Serve the client-side application."""
    with open('client/index.html') as f:
        return web.Response(text=f.read(), content_type='text/html')

# Triggered when a client connects to our socket. 
@server_io.event
def connect(sid, socket):    
    print(sid, 'connected')

# Triggered when a client disconnects from our socket
@server_io.event
def disconnect(sid):
    print(sid, 'disconnected')

@server_io.event
async def chat_message(sid, data):
    print("message ", data)

@server_io.event
def get_name(sid, data):
    """Takes a first name, grabs corresponding last name, and sends it back to the client

    Key arguments:
    sid - the session_id, which is unique to each client
    data - payload sent from the client
    """
    
    print(data)
    
    server_io.emit("name", {'name': dict_names[data["name"]]}, to=sid)

app.router.add_get('/', index)

if __name__ == '__main__':
    web.run_app(app)
