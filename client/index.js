const client_io = io();
  const buttons = document.getElementsByTagName("button");

  client_io.on('connect', () => {
    console.log("connected")

    for (i=0; i<buttons.length; i++) {
      // add click event listener to all buttons
      buttons[i].addEventListener('click', function () {
      // emit hero to server
        client_io.emit("get_name", { name: this.id})            
      })
    }    
  })

  client_io.on('disconnect', () => {
    console.log("disconnected")
  })

  client_io.on("name", (data) => {
    alert(data.dict_name)
  })